# Compiler Monitor

Captures Windows compilation commands and generates `compile_commands.json` for clangd and other IDE tools.

## Installation

Download the latest release from the [GitHub Releases page](https://github.com/functor-cat/compiler_monitor/releases).

Extract `compiler_monitor.exe` and add it to your PATH, or run it directly.

## Quick Start

**Step 1: Record compilation commands**
```bash
# Start monitoring (monitors cl.exe by default)
compiler_monitor.exe record

# In another terminal, build your project
cmake --build . --config Debug

# Stop recording with Ctrl+C
```

**Step 2: Collect into compile_commands.json**
```bash
compiler_monitor.exe collect
```

That's it! Your `compile_commands.json` is ready.

## Usage

### Recording

```bash
# Monitor cl.exe (default)
compiler_monitor.exe record

# Monitor clang instead
compiler_monitor.exe record --pattern "clang.exe"

# Custom cache directory
compiler_monitor.exe record --cache-dir my_cache
```

While recording runs, build your project in another terminal. Press **Ctrl+C** when done.

### Collecting

```bash
# Generate compile_commands.json
compiler_monitor.exe collect

# Custom output location
compiler_monitor.exe collect --output path/to/compile_commands.json

# Read from custom cache directory
compiler_monitor.exe collect --cache-dir my_cache
```

### Aliases

Use `r` for record and `c` for collect:
```bash
compiler_monitor.exe r
compiler_monitor.exe c
```

## How It Works

Monitors Windows processes via WMI to capture:
- Compiler command lines (cl.exe, clang.exe, etc.)
- Working directories
- Response file contents (inlined before deletion)

Records each compilation to a separate file for speed, then merges into `compile_commands.json` when you collect.

## Requirements

- Windows

## Building from Source

Requires Rust 1.70 or later.

```bash
# Clone the repository
git clone https://github.com/functor-cat/compiler_monitor.git
cd compiler_monitor

# Build release binary
cargo build --release

# Binary will be at target/release/compiler_monitor.exe
```

## License

MIT

