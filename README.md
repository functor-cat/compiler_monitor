# Compiler Monitor

A Windows-based compiler process monitor that captures compilation commands and generates `compile_commands.json` for use with IDE tools, clangd, and other development tools.

## How It Works

This tool uses **WMI (Windows Management Instrumentation)** for process monitoring, similar to Process Monitor's ETW approach. It monitors process creation events in real-time and captures:

- Process command line arguments
- Working directory
- Response file contents (inlined before they're deleted)

## Features

- ✅ **Real-time monitoring** using Windows WMI
- ✅ **Response file handling** - automatically detects `@file.rsp` arguments and inlines them
- ✅ **Response file caching** - saves response files before build systems delete them
- ✅ **Pattern matching** - monitor specific compilers (cl.exe, clang.exe, etc.)
- ✅ **Fast recording** - each compilation saved to individual file (no JSON overhead)
- ✅ **Two-step workflow** - record fast, collect once
- ✅ **Standards compliant** - generates JSON Compilation Database format

## Usage

### Two-Step Workflow

The compiler monitor now uses a two-step approach for better performance:

**Step 1: Record** - Monitor and save each compilation to individual files
```bash
# Start recording (monitor cl.exe)
.\target\release\compiler_monitor.exe record

# Or use the alias
.\target\release\compiler_monitor.exe r

# Monitor clang instead
.\target\release\compiler_monitor.exe r --pattern "clang.exe"
```

While recording is active, build your project in another terminal:
```bash
cd test\build
cmake --build . --config Debug
```

Press **Ctrl+C** to stop recording when done.

**Step 2: Collect** - Merge all recorded commands into compile_commands.json
```bash
# Collect all commands
.\target\release\compiler_monitor.exe collect

# Or use the alias
.\target\release\compiler_monitor.exe c

# Custom output location
.\target\release\compiler_monitor.exe c --output my_compile_commands.json
```

### Why Two Steps?

- **Fast recording**: Each compilation is saved to its own file (no JSON merging overhead)
- **No slowdown**: Recording stays fast even with thousands of compilations
- **One-time merge**: Collection happens once at the end, not on every compilation

### Basic Usage

```bash
# Terminal 1: Start recording
cargo run --release -- record

# Terminal 2: Build your project
cd test\build
cmake --build . --config Debug

# Terminal 1: Stop recording (Ctrl+C), then collect
cargo run --release -- collect
```

### Running the Integration Test

A full integration test is provided that:
- Builds a CMake project with Visual Studio generator
- Monitors all cl.exe invocations in parallel
- Saves each command to cache
- Collects and validates the generated compile_commands.json

```bash
# Build and run the integration test
cargo run --release --bin integration_test
```

The test project is located in the `test/` directory and includes multiple C++ source files to generate realistic compile commands.

## Requirements

- **Windows** (uses Windows-specific APIs)
- **Rust** 1.70 or later
- **CMake** (for integration test)
- **Visual Studio** with MSVC (for integration test)

## Command Line Options

### Record Mode (alias: r)
```
Options:
  -p, --pattern <PATTERN>      Process name pattern to monitor [default: cl.exe]
  -c, --cache-dir <CACHE_DIR>  Directory to save recorded commands [default: .compiler_monitor_cache]
  -h, --help                   Print help
```

### Collect Mode (alias: c)
```
Options:
  -c, --cache-dir <CACHE_DIR>  Directory containing recorded commands [default: .compiler_monitor_cache]
  -o, --output <OUTPUT>        Output file for compile_commands.json [default: compile_commands.json]
  -h, --help                   Print help
```

## Project Structure

```
compiler_monitor/
├── src/
│   └── main.rs              # Main compiler monitor application
├── integration_test.rs       # Integration test with threading
├── test/                     # CMake test project
│   ├── CMakeLists.txt
│   └── src/
│       ├── main.cpp
│       ├── mathops.cpp/h
│       ├── utils.cpp/h
│       └── calculator.cpp/h
├── .compiler_monitor_cache/  # Individual command files (created on record)
├── Cargo.toml
└── README.md
```

## Example Output

```json
[
  {
    "directory": "C:\\projects\\myapp\\src",
    "command": "cl.exe /c /Zi /Od /I..\\include main.cpp",
    "file": "C:\\projects\\myapp\\src\\main.cpp",
    "arguments": ["cl.exe", "/c", "/Zi", "/Od", "/I..\\include", "main.cpp"]
  }
]
```

## License

MIT

