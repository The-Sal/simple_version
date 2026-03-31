# Simple Version

A Rust CLI tool for automated semantic versioning based on code changes.

## Overview

Simple Version tracks changes to Rust functions and structs to automatically manage semantic versioning (MAJOR.MINOR.PATCH). It parses Rust source files, computes hashes of their definitions, and determines when versions need to be bumped.

## Features

- **Automated Version Bumping**: Automatically detects code changes and increments version numbers
  - **Patch** (0.0.1→0.0.2): Hash changes in existing functions/structs
  - **Minor** (0.1.0→0.2.0): Added or removed symbols
  - **Major** (1.0.0→2.0.0): Manual major version bump
- **Changelog Generation**: Automatically appends changes to `changelog.txt`
- **Gitignore Support**: Respects `.gitignore` patterns when scanning
- **Rust AST Parsing**: Uses `syn` crate for accurate code analysis

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Initialize version tracking (creates version.json)
cargo run -- init

# Check for changes and auto-bump version
cargo run -- bump

# Force a major version bump
cargo run -- major

# Scan a specific directory
cargo run -- init /path/to/project
```

## Files

- `versionx.json` - Stores current version and symbol hashes (auto-generated)
- `changelog.txt` - Accumulates version history (auto-generated)

## How It Works

1. **Scanning**: Recursively scans `.rs` files respecting `.gitignore`
2. **Parsing**: Extracts functions and structs using Rust's syn parser
3. **Hashing**: Computes SHA256 hashes of each symbol's source code
4. **Comparison**: Compares with stored hashes from `versionx.json`
5. **Versioning**: Bumps version based on change type:
   - Only hash changes → patch increment
   - Additions/removals → minor increment (patch resets)
   - Manual major command → major increment

## Version Rules

| Change Type | Version Impact |
|-------------|----------------|
| Function/struct modified | Patch +1 |
| Function/struct added | Minor +1, Patch 0 |
| Function/struct removed | Minor +1, Patch 0 |
| Manual major bump | Major +1, Minor 0, Patch 0 |

## Dependencies

- `syn` - Rust code parsing
- `sha2` - SHA256 hashing
- `serde` - JSON serialization
- `clap` - CLI interface
- `walkdir` - Directory traversal
- `anyhow` - Error handling
- `hex` - Hash encoding

## Example

```bash
# First run - initialize
cargo run -- init
# Output: Initialized versionx.json with version 0.0.0

# After modifying code
cargo run -- bump
# Output: Bumped version: 0.0.0 -> 0.0.1
#         Updated versionx.json and appended to changelog.txt
```

## License

MIT