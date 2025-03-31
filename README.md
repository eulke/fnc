# FNC - Fast Version Control and Release Tool

FNC (Fast Next Change) is a command-line tool for managing versions, releases, and deployments across multiple project types.

## Features

- **Version Management**: Increment semantic versions (major, minor, patch) across multiple project types
- **Multi-ecosystem Support**: Works with JavaScript/Node.js, Rust, and Python projects
- **Monorepo Support**: Synchronize versions across packages in a monorepo
- **Changelog Management**: Automatically update CHANGELOG.md files
- **Git Integration**: Create release/hotfix branches and manage deployment workflows

## Installation

```bash
# Install from source
cargo install --path cli
```

## Usage

### Deploy a New Version

```bash
# Create a release with a minor version bump
fnc deploy release minor

# Create a hotfix with a patch version bump
fnc deploy hotfix patch
```

### Fix Package Versions in a Monorepo

```bash
# Synchronize versions across all packages
fnc fix-package-version

# Specify a directory to analyze
fnc fix-package-version --dir ./path/to/monorepo
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