# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Build and Test
```bash
# Build all workspace crates
cargo build

# Run all tests
cargo test

# Run tests for specific crate
cargo test -p cli
cargo test -p version
cargo test -p http-diff
cargo test -p changelog
cargo test -p git

# Run specific test by name
cargo test test_name

# Build and run the binary
cargo run -- --help
```

### Code Quality
```bash
# Check code without building
cargo check

# Run linter
cargo clippy

# Format code
cargo fmt
```

### Project Management
```bash
# Run the CLI tool
cargo run -- deploy -i
cargo run -- fix package-versions
cargo run -- sync-versions --source ./main --targets ./sub1,./sub2
cargo run -- http-diff --environments dev,staging --routes health,users
```

## Architecture Overview

FNC is a Rust workspace containing 5 main crates:

### Core Architecture
- **cli**: Main binary and command-line interface using Clap. Entry point in `cli/src/main.rs`
- **version**: Cross-ecosystem version management (JS/Node.js, Rust, Python) with semantic versioning
- **git**: Git repository operations and branch management
- **changelog**: CHANGELOG.md file parsing and manipulation
- **http-diff**: Multi-environment HTTP testing and response comparison

### Key Design Patterns
- **Workspace Structure**: Each crate has its own `Cargo.toml` and focused responsibilities
- **Error Handling**: Custom error types with context using Result types throughout
- **Trait-Based Architecture**: Especially in http-diff crate with traits for HttpClient, ResponseComparator, etc.
- **Cross-Platform**: Built for macOS (both x86_64 and aarch64) with GitHub Actions CI/CD

### HTTP-Diff Module Architecture
The http-diff crate follows clean architecture principles:
- **Traits**: Core abstractions in `traits.rs` (HttpClient, ResponseComparator, TestRunner, etc.)
- **Config**: Environment and route configuration management
- **Execution**: Test running and progress tracking
- **Comparison**: Response comparison and analysis logic
- **Renderers**: Multiple output formats (CLI, TUI, HTML reports)
- **Analysis**: Error classification and grouping

### Key Components
- **Version Management**: Supports JavaScript (package.json), Rust (Cargo.toml), and Python (pyproject.toml/setup.py) ecosystems
- **Monorepo Support**: Can synchronize versions across different project types
- **Git Integration**: Automated release/hotfix branch creation with version bumping
- **Interactive CLI**: Dialog-based workflows using interactive prompts
- **HTTP Testing**: Multi-environment API testing with response comparison and difference reporting

### Testing Strategy
- Unit tests in each crate's `tests/` directory
- Integration tests for cross-crate functionality  
- Mock implementations in http-diff for testing (MockHttpClient, MockTestRunner)
- Example HTTP configurations in `http-diff/examples/`

## Important Implementation Details

### Version Synchronization
The version crate can detect project types automatically and synchronize versions across different ecosystems in monorepos.

### HTTP-Diff Configuration
Uses TOML configuration files (`http-diff.toml`) for environment and route definitions, with CSV user data files for parameterized testing.

### Error Context
All crates use contextual error reporting - errors include detailed context about what operation failed and why.

### TUI vs CLI Output
HTTP-diff module automatically detects TTY and switches between terminal UI and command-line output modes.