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

##### Deploy Command Options
```bash
fnc deploy [deploy_type] [version]
```

Options:
- `DEPLOY_TYPE`: Specify the type of deployment (required)
  - `hotfix`: For quick fixes to the production version
  - `release`: For planned releases with new features
- `VERSION`: Specify the version increment type (optional, defaults to `patch`)
  - `patch`: Increment the patch version (e.g., 1.0.0 -> 1.0.1)
  - `minor`: Increment the minor version (e.g., 1.0.0 -> 1.1.0)
  - `major`: Increment the major version (e.g., 1.0.0 -> 2.0.0)

Example:
```bash
fnc deploy release minor
```

This command will create a new release branch, increment the minor version, and prepare for deployment.