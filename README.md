# FNC - Fast Version Control and Release Tool

FNC (Finance) is a command-line tool for managing versions, releases, and deployments across multiple project types for finance team.

## Features

- **Version Management**: Increment semantic versions (major, minor, patch) across multiple project types
- **Multi-ecosystem Support**: Works with JavaScript/Node.js, Rust, and Python projects
- **Monorepo Support**: Synchronize versions across packages in a monorepo
- **Changelog Management**: Automatically update CHANGELOG.md files
- **Git Integration**: Create release/hotfix branches and manage deployment workflows
- **Interactive Mode**: Dialog-based selection for deployment options

## Installation

```bash
# Install from source
cargo install --path cli
```

## Usage

### Deploy a New Version

Create release or hotfix branches with automated version bumping:

```bash
# Create a release with interactive prompts
fnc deploy -i

# Create a release with a minor version bump
fnc deploy release minor

# Create a hotfix with a patch version bump
fnc deploy hotfix patch

# Enable verbose output
fnc deploy release minor --verbose

# Force deployment even with uncommitted changes (dev only)
fnc deploy release minor --force
```

### Fix Package Versions in a Monorepo

```bash
# Fix package versions in a JavaScript monorepo
fnc fix package-versions

# Specify a directory to analyze
fnc fix package-versions --dir ./path/to/monorepo

# Enable verbose output
fnc fix package-versions --verbose
```

### Synchronize Versions Across Projects

```bash
# Sync versions using a source project and targets
fnc sync-versions --source ./main-project --targets ./project1,./project2

# Auto-discover projects in subdirectories
fnc sync-versions --source ./main-project --targets ./projects --discover

# Set maximum discovery depth (default: 3)
fnc sync-versions --source ./main-project --targets ./projects --discover --max-depth 5
```

### Upgrade FNC

```bash
# Upgrade FNC to the latest version
fnc upgrade

# Force upgrade in development environment
fnc upgrade --force
```

## Supported Ecosystems

- **JavaScript/Node.js**: Works with `package.json` files
- **Rust**: Works with `Cargo.toml` files
- **Python**: Works with `pyproject.toml` or `setup.py` files

## Development

### Project Structure

This project is organized as a Rust workspace with multiple crates:

- `cli`: Command-line interface and main application logic
- `version`: Core version management functionality
- `git`: Git repository interactions
- `changelog`: CHANGELOG.md file management

### Building the Project

```bash
# Build all packages
cargo build

# Run tests
cargo test
```

### Contributing

1. Fork the repository
2. Create your feature branch: `git checkout -b feature/amazing-feature`
3. Commit your changes: `git commit -m 'Add some amazing feature'`
4. Push to the branch: `git push origin feature/amazing-feature`
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.