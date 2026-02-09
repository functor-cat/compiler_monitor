// Compiler Monitor - ETW-based Process Monitoring Tool
//
// This tool monitors compiler process creation on Windows and generates compile_commands.json
// for use with IDE tools, clangd, and other development tools.
//
// ## Technical Approach (Process Monitor Style)
//
// Process Monitor uses ETW (Event Tracing for Windows) with a kernel-mode driver to capture
// system events. This tool uses a similar approach but at user-mode level:
//
// 1. **WMI (Windows Management Instrumentation)** - Queries Win32_Process for process information
// 2. **Process Snapshots** - Uses CreateToolhelp32Snapshot to enumerate running processes
// 3. **Command Line Capture** - Retrieves full command line from each process via WMI
// 4. **Response File Handling** - Detects and inlines MSVC response files (@file.rsp)
//
// ### Why Not Full ETW Kernel Tracing?
//
// Full kernel-mode ETW (Microsoft-Windows-Kernel-Process provider) requires:
// - Administrator privileges
// - More complex implementation with kernel event parsing
// - Higher security permissions
//
// The WMI approach provides:
// - Works without admin rights in most cases
// - Simpler implementation
// - Good enough for compiler monitoring (50ms polling is adequate)
//
// ### Response File Handling
//
// MSVC and other compilers use response files to handle long command lines:
//   cl.exe @C:\Temp\abc123.rsp /Fo"output.obj"
//
// Build systems often delete these files immediately after compilation.
// This tool:
// 1. Detects @file.rsp patterns in command lines
// 2. Reads the response file contents
// 3. Saves a copy to the cache directory (timestamped)
// 4. Inlines the contents into the command line
//
// This ensures compile_commands.json contains complete, self-contained commands.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wmi::{COMLibrary, WMIConnection, Variant};
use windows::Win32::Foundation::*;
use windows::Win32::System::Diagnostics::ToolHelp::*;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use ntapi::ntpsapi::{NtQueryInformationProcess, ProcessBasicInformation, PROCESS_BASIC_INFORMATION};
use ntapi::ntpebteb::PEB;
use ntapi::ntrtl::RTL_USER_PROCESS_PARAMETERS;

/// Command line arguments for the compiler monitor
#[derive(Parser, Debug)]
#[command(author, version, about = "Monitor compiler processes and generate compile_commands.json", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Record compiler invocations to cache (alias: r)
    #[command(alias = "r")]
    Record {
        /// Process name pattern to monitor (e.g., "cl.exe", "clang.exe")
        #[arg(short, long, default_value = "cl.exe")]
        pattern: String,

        /// Directory to save recorded commands
        #[arg(short, long, default_value = ".compiler_monitor_cache")]
        cache_dir: PathBuf,
    },
    /// Collect recorded commands into compile_commands.json (alias: c)
    #[command(alias = "c")]
    Collect {
        /// Directory containing recorded commands
        #[arg(short, long, default_value = ".compiler_monitor_cache")]
        cache_dir: PathBuf,

        /// Output file for compile_commands.json
        #[arg(short, long, default_value = "compile_commands.json")]
        output: PathBuf,
    },
}

/// A single compile command entry in JSON Compilation Database format
/// See: https://clang.llvm.org/docs/JSONCompilationDatabase.html
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompileCommand {
    directory: String,
    command: String,
    file: String,
}

/// Main compiler monitoring structure
/// 
/// Monitors process creation and captures compiler invocations that match the specified pattern.
/// Handles response file inlining and saves individual command files to cache.
struct CompilerMonitor {
    pattern: Regex,
    cache_dir: PathBuf,
    command_counter: Arc<Mutex<u64>>,
    response_counter: Arc<Mutex<u64>>,
}

impl CompilerMonitor {
    fn new(pattern: String, cache_dir: PathBuf) -> Result<Self> {
        let regex_pattern = pattern
            .replace(".", r"\.")
            .replace("*", ".*")
            .replace("?", ".");
        let regex = Regex::new(&format!("(?i)^{}$", regex_pattern))
            .context("Failed to compile regex pattern")?;

        // Create cache directory
        fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

        // Find the highest existing command number to continue from
        let highest_cmd_num = Self::find_highest_command_number(&cache_dir);
        let highest_rsp_num = Self::find_highest_response_number(&cache_dir);

        Ok(Self {
            pattern: regex,
            cache_dir,
            command_counter: Arc::new(Mutex::new(highest_cmd_num)),
            response_counter: Arc::new(Mutex::new(highest_rsp_num)),
        })
    }

    fn find_highest_command_number(cache_dir: &Path) -> u64 {
        let mut highest = 0u64;

        if let Ok(entries) = fs::read_dir(cache_dir) {
            let command_regex = Regex::new(r"command_(\d+)\.json$").unwrap();
            
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if let Some(caps) = command_regex.captures(filename) {
                        if let Ok(num) = caps[1].parse::<u64>() {
                            highest = highest.max(num);
                        }
                    }
                }
            }
        }

        highest
    }

    fn find_highest_response_number(cache_dir: &Path) -> u64 {
        let mut highest = 0u64;

        if let Ok(entries) = fs::read_dir(cache_dir) {
            let response_regex = Regex::new(r"response_(\d+)\.rsp$").unwrap();
            
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if let Some(caps) = response_regex.captures(filename) {
                        if let Ok(num) = caps[1].parse::<u64>() {
                            highest = highest.max(num);
                        }
                    }
                }
            }
        }

        highest
    }

    fn process_creation_callback(
        &self,
        process_name: &str,
        command_line: &str,
        working_dir: &str,
    ) -> Result<()> {
        if !self.pattern.is_match(process_name) {
            return Ok(());
        }

        println!("✓ Detected: {} in {}", process_name, working_dir);
        println!("  Command: {}", command_line);

        // Parse and inline response files
        let expanded_command = self.expand_response_files(command_line, working_dir)?;

        // Extract all source files from command line
        let source_files = self.extract_all_source_files(&expanded_command, working_dir);

        if source_files.is_empty() {
            println!("  ⚠ Warning: No source files found in command");
            return Ok(());
        }

        println!("  Found {} source file(s)", source_files.len());

        // Create one entry per source file
        for source_file in source_files {
            let compile_cmd = CompileCommand {
                directory: working_dir.to_string(),
                command: expanded_command.clone(),
                file: source_file.clone(),
            };

            // Save to individual file in cache
            let mut counter = self.command_counter.lock().unwrap();
            *counter += 1;
            let filename = format!("command_{:06}.json", *counter);
            let filepath = self.cache_dir.join(&filename);
            
            let json = serde_json::to_string_pretty(&compile_cmd)
                .context("Failed to serialize compile command")?;
            fs::write(&filepath, json)
                .with_context(|| format!("Failed to write to {}", filepath.display()))?;
            
            println!("  Saved: {} -> {}", 
                PathBuf::from(&source_file).file_name().unwrap_or_default().to_string_lossy(), 
                filepath.display());
        }

        Ok(())
    }

    fn expand_response_files(&self, command_line: &str, working_dir: &str) -> Result<String> {
        let mut result = command_line.to_string();
        let response_file_regex = Regex::new(r"@([^\s]+)").unwrap();

        // Find all response files
        let matches: Vec<_> = response_file_regex
            .captures_iter(command_line)
            .map(|cap| cap[1].to_string())
            .collect();

        for response_file_path in matches {
            // Resolve the response file path
            let mut full_path = PathBuf::from(&response_file_path);
            if !full_path.is_absolute() {
                full_path = PathBuf::from(working_dir).join(&response_file_path);
            }

            // Read response file contents - try multiple encodings
            // MSVC response files can be UTF-8, UTF-16, or Windows-1252
            let contents = match std::fs::read(&full_path) {
                Ok(bytes) => {
                    // Try UTF-8 first
                    if let Ok(s) = String::from_utf8(bytes.clone()) {
                        s
                    } 
                    // Try UTF-16 LE (common for MSVC)
                    else if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
                        String::from_utf16_lossy(
                            &bytes[2..]
                                .chunks_exact(2)
                                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                                .collect::<Vec<u16>>()
                        )
                    }
                    // Try UTF-16 LE without BOM
                    else if bytes.len() % 2 == 0 {
                        String::from_utf16_lossy(
                            &bytes
                                .chunks_exact(2)
                                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                                .collect::<Vec<u16>>()
                        )
                    }
                    // Fallback to lossy UTF-8
                    else {
                        String::from_utf8_lossy(&bytes).to_string()
                    }
                }
                Err(e) => {
                    println!(
                        "  ⚠ Warning: Could not read response file {}: {}",
                        full_path.display(),
                        e
                    );
                    continue;
                }
            };

            // Save response file to cache
            self.save_response_file(&full_path, &contents)?;

            // Inline the contents
            let cleaned_contents = contents
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ");

            result = result.replace(&format!("@{}", response_file_path), &cleaned_contents);
            println!("  ✓ Inlined response file: {}", full_path.display());
        }

        Ok(result)
    }

    fn save_response_file(&self, _path: &Path, contents: &str) -> Result<()> {
        let mut counter = self.response_counter.lock().unwrap();
        *counter += 1;
        
        let cache_filename = format!("response_{:06}.rsp", *counter);
        let cache_path = self.cache_dir.join(&cache_filename);

        fs::write(&cache_path, contents)
            .with_context(|| format!("Failed to save response file to {}", cache_path.display()))?;

        println!("  [RSP] Saved: {}", cache_path.display());

        Ok(())
    }

    fn extract_all_source_files(&self, command: &str, working_dir: &str) -> Vec<String> {
        // Look for common source file extensions
        let source_extensions = [".c", ".cpp", ".cc", ".cxx", ".c++", ".C"];
        
        // Use proper argument parsing to handle quoted paths
        let args = self.parse_arguments(command);
        let mut source_files = Vec::new();

        for arg in args {
            // Strip quotes and check for source file extensions
            let clean_arg = arg.trim_matches('"');
            let lower = clean_arg.to_lowercase();
            
            if source_extensions.iter().any(|ext| lower.ends_with(ext)) {
                // Make it absolute if relative
                let path = PathBuf::from(clean_arg);
                let absolute_path = if path.is_absolute() {
                    clean_arg.to_string()
                } else {
                    PathBuf::from(working_dir)
                        .join(clean_arg)
                        .to_string_lossy()
                        .to_string()
                };
                source_files.push(absolute_path);
            }
        }

        source_files
    }

    fn parse_arguments(&self, command: &str) -> Vec<String> {
        // Simple argument parsing - split on spaces but respect quotes
        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut in_quotes = false;
        
        for c in command.chars() {
            match c {
                '"' => in_quotes = !in_quotes,
                ' ' if !in_quotes => {
                    if !current_arg.is_empty() {
                        args.push(current_arg.clone());
                        current_arg.clear();
                    }
                }
                _ => current_arg.push(c),
            }
        }
        
        if !current_arg.is_empty() {
            args.push(current_arg);
        }
        
        args
    }
}

// Process creation via WMI Event Subscription
// This uses Windows WMI to subscribe to process creation events
// Similar to Process Monitor's approach but at user-mode level
fn monitor_with_wmi(monitor: Arc<CompilerMonitor>) -> Result<()> {
    println!("Starting WMI-based process monitor...");
    println!("Note: Capturing process creation events in real-time");
    println!("Press Ctrl+C to stop monitoring\n");

    let com_lib = COMLibrary::new().context("Failed to initialize COM library")?;
    let wmi_con = WMIConnection::new(com_lib.into())
        .context("Failed to create WMI connection")?;

    println!("✓ Connected to WMI");
    println!("✓ Monitoring process creation...\n");

    // Use polling with process snapshots
    // For true event-based monitoring, you'd use WMI event subscriptions with
    // __InstanceCreationEvent on Win32_Process, but that requires more complex COM handling
    
    let mut known_processes = std::collections::HashSet::new();
    let mut counter = 0u64;

    loop {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .context("Failed to create process snapshot")?;

            if snapshot.is_invalid() {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            let mut pe = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut pe).is_ok() {
                loop {
                    let pid = pe.th32ProcessID;
                    let process_name = String::from_utf16_lossy(
                        &pe.szExeFile[..pe
                            .szExeFile
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(pe.szExeFile.len())],
                    );

                    // Check if this matches our pattern
                    if monitor.pattern.is_match(&process_name) {
                        let key = format!("{}:{}", pid, process_name);
                        if !known_processes.contains(&key) {
                            known_processes.insert(key.clone());
                            counter += 1;

                            // Get full process information via WMI
                            if let Ok((cmd_line, work_dir)) = get_process_info_wmi(&wmi_con, pid) {
                                if !cmd_line.is_empty() {
                                    let _ = monitor.process_creation_callback(
                                        &process_name,
                                        &cmd_line,
                                        &work_dir,
                                    );
                                    println!("  [{}] Captured compilation command\n", counter);
                                }
                            }
                        }
                    }

                    if Process32NextW(snapshot, &mut pe).is_err() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
        }

        // Cleanup old entries periodically to prevent memory growth
        if known_processes.len() > 10000 {
            known_processes.clear();
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Get the current working directory of a process using NtQueryInformationProcess
/// This reads the PEB (Process Environment Block) to get the real working directory
fn get_process_working_directory(pid: u32) -> Option<String> {
    unsafe {
        // Open the process with query and read permissions
        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        ).ok()?;

        // Query basic process information to get PEB address
        let mut pbi: PROCESS_BASIC_INFORMATION = std::mem::zeroed();
        let mut return_length: u32 = 0;
        
        let status = NtQueryInformationProcess(
            handle.0 as *mut _,
            ProcessBasicInformation,
            &mut pbi as *mut _ as *mut _,
            std::mem::size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            &mut return_length,
        );
        
        if status != 0 {
            let _ = CloseHandle(handle);
            return None;
        }

        // Read the PEB from the target process
        let mut peb: PEB = std::mem::zeroed();
        let mut bytes_read: usize = 0;
        
        let success = ReadProcessMemory(
            handle,
            pbi.PebBaseAddress as *const _,
            &mut peb as *mut _ as *mut _,
            std::mem::size_of::<PEB>(),
            Some(&mut bytes_read),
        );
        
        if success.is_err() || bytes_read != std::mem::size_of::<PEB>() {
            let _ = CloseHandle(handle);
            return None;
        }

        // Read the RTL_USER_PROCESS_PARAMETERS from the target process
        let mut upp: RTL_USER_PROCESS_PARAMETERS = std::mem::zeroed();
        
        let success = ReadProcessMemory(
            handle,
            peb.ProcessParameters as *const _,
            &mut upp as *mut _ as *mut _,
            std::mem::size_of::<RTL_USER_PROCESS_PARAMETERS>(),
            Some(&mut bytes_read),
        );
        
        if success.is_err() || bytes_read != std::mem::size_of::<RTL_USER_PROCESS_PARAMETERS>() {
            let _ = CloseHandle(handle);
            return None;
        }

        // Read the CurrentDirectoryPath string from the target process
        let path_length = upp.CurrentDirectory.DosPath.Length as usize;
        if path_length == 0 || path_length > 32768 {
            let _ = CloseHandle(handle);
            return None;
        }

        let mut path_buffer: Vec<u16> = vec![0u16; path_length / 2 + 1];
        
        let success = ReadProcessMemory(
            handle,
            upp.CurrentDirectory.DosPath.Buffer as *const _,
            path_buffer.as_mut_ptr() as *mut _,
            path_length,
            Some(&mut bytes_read),
        );
        
        let _ = CloseHandle(handle);
        
        if success.is_err() || bytes_read != path_length {
            return None;
        }

        // Convert to String, removing trailing backslash if present
        let mut path = String::from_utf16_lossy(&path_buffer[..path_length / 2]);
        if path.ends_with('\\') {
            path.pop();
        }
        
        Some(path)
    }
}

fn get_process_info_wmi(wmi_con: &WMIConnection, pid: u32) -> Result<(String, String)> {
    // Query WMI for process information (command line)
    let query = format!("SELECT CommandLine FROM Win32_Process WHERE ProcessId = {}", pid);
    
    let results: Vec<std::collections::HashMap<String, Variant>> = wmi_con
        .raw_query(&query)
        .unwrap_or_default();

    if results.is_empty() {
        return Ok((String::new(), String::new()));
    }

    let result = &results[0];
    
    let cmd_line = result
        .get("CommandLine")
        .and_then(|v| match v {
            Variant::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();

    // Get the real working directory using NtQueryInformationProcess
    let work_dir = get_process_working_directory(pid)
        .unwrap_or_else(|| {
            // Fallback to current directory if we can't read the process
            std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    Ok((cmd_line, work_dir))
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           Compiler Monitor (ETW-based)                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    match args.command {
        Commands::Record { pattern, cache_dir } => {
            println!("Mode: RECORD");
            println!("Configuration:");
            println!("  Pattern:     {}", pattern);
            println!("  Cache Dir:   {}", cache_dir.display());
            println!();

            let monitor = Arc::new(CompilerMonitor::new(pattern, cache_dir)?);

            // Note: Full ETW kernel-mode monitoring requires administrator privileges
            // This implementation uses WMI/process snapshot as a fallback
            monitor_with_wmi(monitor)?;
        }
        Commands::Collect { cache_dir, output } => {
            println!("Mode: COLLECT");
            println!("Configuration:");
            println!("  Cache Dir:   {}", cache_dir.display());
            println!("  Output:      {}", output.display());
            println!();

            collect_commands(&cache_dir, &output)?;
        }
    }

    Ok(())
}

fn collect_commands(cache_dir: &Path, output_path: &Path) -> Result<()> {
    println!("Collecting commands from cache...");

    if !cache_dir.exists() {
        anyhow::bail!("Cache directory does not exist: {}", cache_dir.display());
    }

    let mut commands = Vec::new();
    let mut count = 0;

    // Read all JSON files from cache directory
    for entry in fs::read_dir(cache_dir).context("Failed to read cache directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            
            let cmd: CompileCommand = serde_json::from_str(&contents)
                .with_context(|| format!("Failed to parse JSON from {}", path.display()))?;
            
            commands.push(cmd);
            count += 1;
        }
    }

    println!("  Found {} command(s)", count);

    // Sort by file path for consistent ordering
    commands.sort_by(|a, b| a.file.cmp(&b.file));

    // Write to output file
    let json = serde_json::to_string_pretty(&commands)
        .context("Failed to serialize commands")?;
    
    fs::write(output_path, json)
        .with_context(|| format!("Failed to write to {}", output_path.display()))?;

    println!("✓ Written to {}", output_path.display());
    println!("✓ Total commands: {}", count);

    Ok(())
}

