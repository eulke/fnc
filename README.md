# fnc (Finance)

`fnc` is a command-line interface (CLI) tool designed to automate repetitive tasks in team related projects. It simplifies the deployment process by handling version increments, branch creation, and changelog updates.

## Features

- Automate deploy flows for releases and hotfixes
- Detect project language automatically
- Increment version numbers (patch, minor, major)
- Create and checkout new branches
- Update package versions
- Integrate with Git for version control operations

### Install

```bash
curl -fsSL https://raw.githubusercontent.com/eulke/fnc/main/install.sh | bash
```

### Post-Installation Setup
After installation, you need to source your shell configuration file to use fnc immediately:

For Bash users:
```bash
source ~/.bashrc
```

For Zsh users:
```bash
source ~/.zshrc
```

### Usage
The basic syntax for using `fnc` is:

```bash
fnc [command] [options]
```

#### Available Commands
Currently, `fnc` supports the following command:

- `deploy`: Automate the deploy flow by creating a branch, incrementing the version, and updating the changelog.

##### Deploy Command
```bash
fnc deploy [OPTIONS] [DEPLOY_TYPE] [VERSION]
```

###### Options
- `-i, --interactive`: Run in interactive mode. When enabled, you'll be guided through the deployment process with prompts.

###### Deploy Types
- `hotfix`: Create a hotfix deployment from the main/master branch
- `release`: Create a release deployment from the default branch

###### Version Types
- `patch`: Increment the patch version (e.g., 1.0.0 -> 1.0.1)
- `minor`: Increment the minor version (e.g., 1.0.0 -> 1.1.0)
- `major`: Increment the major version (e.g., 1.0.0 -> 2.0.0)

###### Examples
```bash
# Interactive deployment
fnc deploy -i

# Create a hotfix with patch version increment
fnc deploy hotfix patch

# Create a release with minor version increment
fnc deploy release minor

# Create a release with major version increment
fnc deploy release major
```

### Supported Languages
The tool automatically detects and supports the following project types:
- Node.js (package.json)
- Rust (Cargo.toml)

### Requirements
- Git installed and configured
- Project must be in a Git repository
- Project must use one of the supported version control formats