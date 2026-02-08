use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║         Compiler Monitor Integration Test                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Clean up old test artifacts
    println!("[Setup] Cleaning up old test files...");
    let _ = std::fs::remove_file("compile_commands.json");
    let _ = std::fs::remove_dir_all(".compiler_monitor_cache");
    let _ = std::fs::remove_dir_all("test/build");
    std::fs::create_dir_all("test/build").expect("Failed to create build directory");
    println!("✓ Cleanup complete\n");

    // Build the monitor if needed
    println!("[Setup] Building compiler monitor...");
    let build_status = Command::new("cargo")
        .args(&["build", "--release"])
        .status()
        .expect("Failed to build compiler monitor");
    
    if !build_status.success() {
        eprintln!("❌ Failed to build compiler monitor");
        std::process::exit(1);
    }
    println!("✓ Compiler monitor built\n");

    // Spawn monitor thread
    println!("[Thread 1] Starting compiler monitor...");
    let _monitor_thread = thread::spawn(|| {
        let status = Command::new("target/release/compiler_monitor.exe")
            .current_dir(".")
            .status();
        
        match status {
            Ok(s) => println!("[Thread 1] Monitor exited with status: {}", s),
            Err(e) => eprintln!("[Thread 1] Monitor error: {}", e),
        }
    });

    // Give the monitor time to start up
    thread::sleep(Duration::from_secs(2));
    println!("✓ Monitor is running\n");

    // Spawn CMake build thread
    println!("[Thread 2] Starting CMake configuration and build...");
    let build_thread = thread::spawn(|| {
        let test_dir = PathBuf::from("test");
        let build_dir = test_dir.join("build");

        // Find Visual Studio installation using vswhere
        println!("[Thread 2] Locating Visual Studio installation...");
        let vswhere_output = Command::new("C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe")
            .args(&[
                "-latest",
                "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-property", "installationPath",
            ])
            .output();

        let vs_path = match vswhere_output {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            _ => {
                eprintln!("[Thread 2] ⚠ Could not find Visual Studio installation");
                String::new()
            }
        };

        let vcvars_path = if !vs_path.is_empty() {
            PathBuf::from(&vs_path).join("VC\\Auxiliary\\Build\\vcvarsall.bat")
        } else {
            PathBuf::from("C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvarsall.bat")
        };

        if !vcvars_path.exists() {
            eprintln!("[Thread 2] ❌ vcvarsall.bat not found at: {}", vcvars_path.display());
            eprintln!("[Thread 2] Visual Studio Developer tools may not be installed");
        } else {
            println!("[Thread 2] ✓ Found vcvarsall.bat");
        }

        // Configure with CMake using Developer Command Prompt environment
        println!("[Thread 2] Configuring CMake project with Visual Studio generator...");
        let configure_cmd = format!(
            "\"{}\" x64 && cd /d {} && cmake .. -G \"Visual Studio 17 2022\" -A x64",
            vcvars_path.display(),
            build_dir.canonicalize().unwrap_or(build_dir.clone()).display()
        );

        let configure_status = Command::new("cmd")
            .args(&["/c", &configure_cmd])
            .status();

        match configure_status {
            Ok(status) if status.success() => {
                println!("[Thread 2] ✓ CMake configuration successful\n");
            }
            Ok(_) => {
                eprintln!("[Thread 2] ❌ CMake configuration failed");
                eprintln!("[Thread 2] Note: You may need Visual Studio 2022 installed");
                eprintln!("[Thread 2] Trying with default generator...");
                
                // Try with default generator as fallback
                let fallback_status = Command::new("cmake")
                    .arg("..")
                    .current_dir(&build_dir)
                    .status()
                    .expect("Failed to run cmake");
                
                if !fallback_status.success() {
                    eprintln!("[Thread 2] ❌ CMake configuration failed with default generator too");
                    return;
                }
            }
            Err(e) => {
                eprintln!("[Thread 2] ❌ Failed to execute cmake: {}", e);
                eprintln!("[Thread 2] Make sure CMake is installed and in PATH");
                return;
            }
        }

        // Build the project
        println!("[Thread 2] Building project...");
        let build_status = Command::new("cmake")
            .args(&[
                "--build", ".",
                "--config", "Debug",
                "--verbose",
            ])
            .current_dir(&build_dir)
            .status();

        match build_status {
            Ok(status) if status.success() => {
                println!("[Thread 2] ✓ Build successful!\n");
            }
            Ok(_) => {
                eprintln!("[Thread 2] ❌ Build failed");
            }
            Err(e) => {
                eprintln!("[Thread 2] ❌ Failed to execute build: {}", e);
            }
        }

        // Run the test executable
        println!("[Thread 2] Running test executable...");
        let exe_path = build_dir.join("Debug").join("CompilerMonitorTest.exe");
        if exe_path.exists() {
            let run_status = Command::new(&exe_path)
                .status();
            
            match run_status {
                Ok(status) if status.success() => {
                    println!("[Thread 2] ✓ Test executable ran successfully\n");
                }
                _ => {
                    eprintln!("[Thread 2] ⚠ Test executable failed or didn't run");
                }
            }
        } else {
            println!("[Thread 2] ⚠ Test executable not found at expected location");
        }

        println!("[Thread 2] Build thread complete");
    });

    // Wait for build to complete
    build_thread.join().expect("Build thread panicked");
    
    // Give monitor a moment to catch any late processes
    thread::sleep(Duration::from_secs(1));

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Analyzing Results                                           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Check if compile_commands.json was created
    if let Ok(contents) = std::fs::read_to_string("compile_commands.json") {
        println!("✓ compile_commands.json created successfully!\n");
        
        // Parse and display statistics
        match serde_json::from_str::<Vec<serde_json::Value>>(&contents) {
            Ok(commands) => {
                println!("Statistics:");
                println!("  Total compile commands: {}", commands.len());
                println!("  Expected: ~7 (main.cpp, math.cpp, utils.cpp, calculator.cpp + library builds)");
                
                if commands.len() > 0 {
                    println!("\nSample commands:");
                    for (i, cmd) in commands.iter().take(3).enumerate() {
                        if let Some(file) = cmd.get("file").and_then(|f| f.as_str()) {
                            println!("  [{}] {}", i + 1, file);
                        }
                        if let Some(command) = cmd.get("command").and_then(|c| c.as_str()) {
                            let short_cmd = if command.len() > 80 {
                                format!("{}...", &command[..80])
                            } else {
                                command.to_string()
                            };
                            println!("      {}", short_cmd);
                        }
                    }
                    if commands.len() > 3 {
                        println!("  ... and {} more", commands.len() - 3);
                    }
                }
                
                // Check for response files
                let with_response_files = commands.iter()
                    .filter(|cmd| {
                        cmd.get("command")
                            .and_then(|c| c.as_str())
                            .map(|s| s.contains("@"))
                            .unwrap_or(false)
                    })
                    .count();
                
                if with_response_files > 0 {
                    println!("\n⚠ Note: {} commands still contain @ (response files not inlined)", with_response_files);
                }
            }
            Err(e) => {
                eprintln!("⚠ Failed to parse JSON: {}", e);
            }
        }

        // Show first entry in detail
        if let Ok(commands) = serde_json::from_str::<Vec<serde_json::Value>>(&contents) {
            if let Some(first) = commands.first() {
                println!("\nFirst entry (formatted):");
                println!("{}", serde_json::to_string_pretty(first).unwrap_or_default());
            }
        }
    } else {
        println!("❌ compile_commands.json was NOT created");
        println!("\nPossible reasons:");
        println!("  - Monitor didn't detect cl.exe processes");
        println!("  - Build used a different compiler");
        println!("  - Processes were too short-lived");
        println!("\nTry running manually:");
        println!("  Terminal 1: .\\target\\release\\compiler_monitor.exe");
        println!("  Terminal 2: cd test\\build && cmake --build . --config Debug");
    }

    // Check cache directory
    if let Ok(entries) = std::fs::read_dir(".compiler_monitor_cache") {
        let count = entries.count();
        if count > 0 {
            println!("\n✓ Response file cache: {} file(s) saved", count);
        }
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Test Complete - Stopping Monitor                           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // The monitor thread is still running, we need to kill it
    // Since we can't easily stop it from here, we'll just exit
    // In a real scenario, you'd run the monitor in a separate process and kill it
    std::process::exit(0);
}
