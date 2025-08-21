# FNC - Fast DevOps Automation Toolkit

[![Rust](https://img.shields.io/badge/built_with-Rust-red)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/eulke/fnc)](https://github.com/eulke/fnc/releases)

FNC is a comprehensive command-line toolkit designed to streamline DevOps workflows across multiple project ecosystems. Built with Rust for performance and reliability, FNC provides powerful automation for version management, release workflows, HTTP testing, and cross-environment validation.

## 🚀 Key Features

### **Multi-Environment HTTP Testing & Comparison**
- Execute identical HTTP requests across multiple environments (dev, staging, prod)
- Advanced response comparison with difference visualization
- Chain request execution with value extraction and dependencies
- Professional HTML reports and interactive terminal UI
- Support for complex authentication flows and API testing scenarios

### **Cross-Ecosystem Version Management**
- Unified version management across JavaScript/Node.js, Rust, and Python projects
- Semantic versioning with automated CHANGELOG.md updates
- Monorepo support with version synchronization
- Git integration for automated release and hotfix workflows

### **Intelligent Project Automation**
- Interactive deployment workflows with branch management
- Package version fixing in JavaScript monorepos
- Cross-project version synchronization with auto-discovery
- Self-updating CLI with development environment detection

## 📦 Installation

### Quick Install
```bash
curl -fsSL https://raw.githubusercontent.com/eulke/fnc/main/install.sh | bash
```

### Manual Installation
1. Download the latest release from [GitHub Releases](https://github.com/eulke/fnc/releases)
2. Extract and add to your PATH
3. Verify installation: `fnc --help`

### Development Build
```bash
git clone https://github.com/eulke/fnc.git
cd fnc
cargo build --release
./target/release/fnc --help
```

---

# 🌐 HTTP-Diff: Multi-Environment API Testing

The HTTP-diff module is FNC's flagship feature for comprehensive API testing and validation across multiple environments. It provides sophisticated capabilities for request chaining, response comparison, and detailed reporting.

## Core Capabilities

### **Multi-Environment Testing**
Execute identical HTTP requests across multiple configured environments and automatically compare responses to identify differences, regressions, or environment-specific issues.

### **Advanced Request Chaining**
Build complex test scenarios with request dependencies, value extraction, and conditional execution. Perfect for authentication flows, data setup, and end-to-end API testing.

### **Professional Reporting**
Generate multiple output formats including interactive terminal UI, structured CLI output, and professional HTML reports suitable for stakeholders and documentation.

## Quick Start

### 1. Initialize Configuration
```bash
# Generate default configuration files
fnc http-diff --init

# This creates:
# - http-diff.toml (main configuration)
# - users.csv (test user data)
```

### 2. Basic Configuration Example
```toml
# http-diff.toml
[environments.dev]
base_url = "https://api-dev.example.com"
headers = { "Accept" = "application/json" }

[environments.staging]
base_url = "https://api-staging.example.com"
headers = { "Accept" = "application/json" }

[environments.prod]
base_url = "https://api.example.com"
headers = { "Accept" = "application/json" }

[[routes]]
name = "health"
method = "GET"
path = "/health"

[[routes]]
name = "user_profile"
method = "GET"
path = "/api/users/{userId}"
headers = { "Authorization" = "Bearer {token}" }
```

### 3. User Data (CSV)
```csv
userId,token,userType
1001,eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...,premium
1002,eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...,basic
```

### 4. Execute Tests
```bash
# Test all environments and routes
fnc http-diff

# Test specific environments
fnc http-diff --environments dev,staging

# Test specific routes
fnc http-diff --routes health,user_profile

# Generate HTML report
fnc http-diff --report results.html

# Force CLI output (disable TUI)
fnc http-diff --no-tui
```

## Configuration Reference

### Environment Configuration

#### Basic Environment Setup
```toml
[environments.development]
base_url = "https://api-dev.company.com"
headers = { 
    "Accept" = "application/json",
    "User-Agent" = "FNC-HttpDiff/1.0",
    "X-Environment" = "development"
}

[environments.production]
base_url = "https://api.company.com"
headers = { 
    "Accept" = "application/json",
    "User-Agent" = "FNC-HttpDiff/1.0"
}
```

#### Global Configuration
```toml
[global]
timeout = 30                    # Request timeout in seconds
max_concurrent = 10             # Maximum concurrent requests
follow_redirects = true         # Follow HTTP redirects
headers = {                     # Headers applied to all requests
    "Accept" = "application/json",
    "User-Agent" = "FNC-HttpDiff/1.0"
}
```

### Route Configuration

#### Simple Routes
```toml
[[routes]]
name = "health_check"
method = "GET"
path = "/health"

[[routes]]
name = "api_status"
method = "GET"
path = "/api/v1/status"
headers = { "X-Health-Check" = "true" }
```

#### Parameterized Routes
```toml
[[routes]]
name = "user_detail"
method = "GET"
path = "/api/users/{userId}"
headers = { "Authorization" = "Bearer {authToken}" }

[[routes]]
name = "create_order"
method = "POST"
path = "/api/orders"
headers = { 
    "Authorization" = "Bearer {authToken}",
    "Content-Type" = "application/json"
}
body = '''
{
    "userId": "{userId}",
    "items": [
        {"productId": "{productId}", "quantity": 1}
    ],
    "shippingAddress": "{address}"
}
'''
```

## Advanced Features

### Request Chaining and Dependencies

Build complex test scenarios where requests depend on previous responses:

```toml
# Step 1: Authentication
[[routes]]
name = "login"
method = "POST"
path = "/auth/login"
headers = { "Content-Type" = "application/json" }
body = '''
{
    "username": "{username}",
    "password": "{password}"
}
'''

# Extract auth token from login response
[[routes.extract]]
name = "auth_token"
type = "json_path"
source = "$.access_token"
required = true

[[routes.extract]]
name = "user_id"
type = "json_path"
source = "$.user.id"
required = true

# Step 2: Get user data (depends on login)
[[routes]]
name = "user_profile"
method = "GET"
path = "/api/users/{user_id}"
headers = { "Authorization" = "Bearer {auth_token}" }
depends_on = ["login"]              # Wait for login to complete
wait_for_extraction = true          # Wait for value extraction

# Extract additional data
[[routes.extract]]
name = "user_email"
type = "json_path"
source = "$.email"
required = false
default_value = "unknown@example.com"

# Step 3: Update profile (depends on user_profile)
[[routes]]
name = "update_profile"
method = "PUT"
path = "/api/users/{user_id}/profile"
headers = { 
    "Authorization" = "Bearer {auth_token}",
    "Content-Type" = "application/json"
}
body = '''
{
    "email": "{user_email}",
    "updatedBy": "{username}"
}
'''
depends_on = ["user_profile"]
wait_for_extraction = true
```

### Value Extraction System

Extract values from responses for use in subsequent requests:

#### JSON Path Extraction
```toml
[[routes.extract]]
name = "order_id"
type = "json_path"
source = "$.data.order.id"
required = true

[[routes.extract]]
name = "total_amount"
type = "json_path"
source = "$.data.order.total"
required = false
default_value = "0.00"
```

#### Regular Expression Extraction
```toml
[[routes.extract]]
name = "transaction_id"
type = "regex"
source = "Transaction ID: ([A-Z0-9]+)"
required = true

[[routes.extract]]
name = "session_token"
type = "regex"
source = "token=([a-f0-9]+)"
required = false
default_value = "no-token"
```

#### Header Extraction
```toml
[[routes.extract]]
name = "request_id"
type = "header"
source = "X-Request-ID"
required = false

[[routes.extract]]
name = "rate_limit"
type = "header"
source = "X-RateLimit-Remaining"
required = false
default_value = "unknown"
```

#### Status Code Extraction
```toml
[[routes.extract]]
name = "response_status"
type = "status_code"
required = true
```

### Conditional Execution

Execute routes conditionally based on user data or extracted values:

```toml
[[routes]]
name = "premium_features"
method = "GET"
path = "/api/premium/features"
headers = { "Authorization" = "Bearer {auth_token}" }

# Only execute for premium users
[routes.conditions]
field = "userType"
operator = "equals"
value = "premium"

[[routes]]
name = "admin_dashboard"
method = "GET"
path = "/api/admin/dashboard"
headers = { "Authorization" = "Bearer {auth_token}" }

# Only execute if user has admin role (from extracted value)
[routes.conditions]
field = "user_role"
operator = "equals"
value = "admin"

[[routes]]
name = "high_value_orders"
method = "GET"
path = "/api/orders/high-value"
headers = { "Authorization" = "Bearer {auth_token}" }

# Only execute if order total is above threshold
[routes.conditions]
field = "total_amount"
operator = "greater_than"
value = "1000.00"
```

#### Supported Condition Operators
- `equals` / `not_equals`: Exact string matching
- `contains` / `not_contains`: Substring matching
- `greater_than` / `less_than`: Numeric comparison
- `exists` / `not_exists`: Check if field exists and is not empty

## Command Line Reference

### Basic Usage
```bash
fnc http-diff [OPTIONS]
```

### Environment Selection
```bash
# Test specific environments
fnc http-diff --environments dev,staging
fnc http-diff -e production

# Test all configured environments (default)
fnc http-diff
```

### Route Filtering
```bash
# Test specific routes
fnc http-diff --routes health,user_profile
fnc http-diff -r login,dashboard

# Test all configured routes (default)
fnc http-diff
```

### Comparison Options
```bash
# Include headers in comparison
fnc http-diff --include-headers

# Include error analysis
fnc http-diff --include-errors

# Choose diff view style
fnc http-diff --diff-view unified        # Default
fnc http-diff --diff-view side-by-side   # Side-by-side comparison
```

### Configuration Files
```bash
# Use custom configuration file
fnc http-diff --config my-config.toml

# Use custom user data file
fnc http-diff --users-file test-data.csv

# Generate default configuration files
fnc http-diff --init
```

### Output Options
```bash
# Generate HTML report
fnc http-diff --report results.html

# Save curl commands for debugging
fnc http-diff --output-file debug-commands.txt

# Force CLI output (disable interactive TUI)
fnc http-diff --no-tui

# Force TUI even when output is redirected
fnc http-diff --force-tui

# Enable verbose logging
fnc http-diff --verbose
```

### Combined Examples
```bash
# Comprehensive test with reporting
fnc http-diff \
    --environments dev,staging,prod \
    --routes critical_endpoints \
    --include-headers \
    --include-errors \
    --report executive-summary.html \
    --verbose

# Debug specific route
fnc http-diff \
    --routes problematic_endpoint \
    --environments dev \
    --output-file debug.txt \
    --no-tui \
    --verbose

# Quick health check
fnc http-diff \
    --routes health \
    --diff-view side-by-side
```

## Output Formats

### Interactive Terminal UI (TUI)
When stdout is a TTY, FNC automatically launches an interactive terminal interface featuring:
- Real-time progress tracking
- Detailed difference visualization
- Error analysis and categorization
- Configuration validation display
- Filtering and navigation capabilities

### Command Line Output
When output is redirected or `--no-tui` is specified:
- Structured text output with color coding
- Summary statistics and error reporting
- Configurable verbosity levels
- Script-friendly formats

### HTML Executive Reports
Professional reports suitable for stakeholders:
- Executive summary with key metrics
- Detailed difference analysis
- Responsive design for various screen sizes
- Embedded styling for standalone distribution

Example HTML report generation:
```bash
fnc http-diff --report quarterly-api-validation.html
```

## Real-World Use Cases

### 1. **Pre-Deployment Validation**
```bash
# Validate API consistency before production deployment
fnc http-diff \
    --environments staging,prod \
    --include-headers \
    --report pre-deployment-validation.html
```

### 2. **Authentication Flow Testing**
```toml
# Complex authentication chain with token refresh
[[routes]]
name = "initial_login"
method = "POST"
path = "/auth/login"
# ... extract tokens ...

[[routes]]
name = "protected_resource"
method = "GET"
path = "/api/protected/data"
depends_on = ["initial_login"]
# ... use extracted tokens ...

[[routes]]
name = "token_refresh"
method = "POST"
path = "/auth/refresh"
depends_on = ["protected_resource"]
# ... refresh expired tokens ...
```

### 3. **Multi-Tenant Testing**
```csv
tenantId,apiKey,environment,userType
tenant1,key123,prod,enterprise
tenant2,key456,prod,standard
tenant3,key789,staging,trial
```

### 4. **Performance Regression Detection**
```bash
# Monitor response times across environments
fnc http-diff \
    --environments baseline,current \
    --routes performance_critical \
    --include-errors \
    --verbose
```

## Best Practices

### Configuration Organization
- Use descriptive environment and route names
- Group related routes logically
- Include comments in TOML files for complex configurations
- Version control your configuration files

### Test Data Management
- Use realistic but anonymized test data
- Rotate API keys and tokens regularly
- Separate test data by environment
- Validate CSV data format before execution

### Error Handling
- Always specify required vs optional extractions
- Provide meaningful default values
- Test dependency chains thoroughly
- Monitor for circular dependencies

### Performance Optimization
- Adjust `max_concurrent` based on target API capacity
- Use appropriate timeout values
- Consider request ordering for optimal execution
- Monitor resource usage during large test runs

---

# 🔧 Version Management & Release Automation

FNC provides comprehensive version management across multiple project ecosystems with automated release workflows.

## Supported Project Types
- **JavaScript/Node.js**: `package.json` files
- **Rust**: `Cargo.toml` files  
- **Python**: `pyproject.toml` and `setup.py` files

## Deploy Commands

### Interactive Deployment
```bash
# Launch interactive deployment wizard
fnc deploy -i
```

### Direct Deployment
```bash
# Create release branch with minor version bump
fnc deploy release minor

# Create hotfix branch with patch version bump
fnc deploy hotfix patch

# Create release with major version bump
fnc deploy release major
```

### Deployment Options
```bash
# Enable verbose output
fnc deploy release minor --verbose

# Force deployment with uncommitted changes (dev only)
fnc deploy release minor --force
```

## Version Synchronization

### Monorepo Version Management
```bash
# Fix package versions in JavaScript monorepo
fnc fix package-versions

# Specify directory to analyze
fnc fix package-versions --dir ./path/to/monorepo

# Enable verbose output
fnc fix package-versions --verbose
```

### Cross-Project Synchronization
```bash
# Sync versions using source and targets
fnc sync-versions --source ./main-project --targets ./project1,./project2

# Auto-discover projects in subdirectories
fnc sync-versions --source ./main-project --targets ./projects --discover

# Set maximum discovery depth
fnc sync-versions --source ./main-project --targets ./projects --discover --max-depth 5
```

## Changelog Management

### Automatic Changelog Updates
```bash
# Fix changelog entries by moving unreleased changes
fnc fix changelog

# Enable verbose output for detailed operations
fnc fix changelog --verbose
```

---

# 🛠 Development & Contributing

## Project Architecture

FNC is organized as a Rust workspace with focused, single-responsibility crates:

### Core Crates
- **`cli`**: Command-line interface and main application logic using Clap
- **`version`**: Cross-ecosystem version management with semantic versioning
- **`git`**: Git repository interactions and branch management
- **`changelog`**: CHANGELOG.md file parsing and manipulation
- **`http-diff`**: Multi-environment HTTP testing and response comparison

### Architecture Principles
- **Clean Architecture**: Separation of concerns with trait-based design
- **Cross-Platform**: Built for macOS (x86_64 and aarch64) with CI/CD
- **Error Context**: Detailed error reporting with actionable context
- **Testing**: Comprehensive unit and integration tests with mock implementations

## Building the Project

### Development Setup
```bash
# Clone repository
git clone https://github.com/eulke/fnc.git
cd fnc

# Build all workspace crates
cargo build

# Run all tests
cargo test

# Run specific crate tests
cargo test -p http-diff
cargo test -p version
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

### Testing Individual Features
```bash
# Test HTTP-diff functionality
cargo run -- http-diff --help
cargo run -- http-diff --environments dev,staging

# Test version management
cargo run -- deploy -i
cargo run -- sync-versions --source ./main --targets ./sub1,./sub2
```

## Contributing Guidelines

### Development Workflow
1. Fork the repository
2. Create feature branch: `git checkout -b feature/amazing-feature`
3. Write tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Run code quality checks: `cargo clippy && cargo fmt`
6. Commit changes: `git commit -m 'Add some amazing feature'`
7. Push to branch: `git push origin feature/amazing-feature`
8. Open a Pull Request

### Code Standards
- Follow Rust community conventions and idioms
- Add comprehensive tests for new features
- Include documentation for public APIs
- Use descriptive error messages with context
- Maintain backwards compatibility when possible

### Testing Requirements
- Unit tests for all core functionality
- Integration tests for cross-crate interactions
- Example configurations for new features
- Performance tests for critical paths

## Self-Updating

### CLI Updates
```bash
# Upgrade to latest version
fnc upgrade

# Force upgrade in development environment
fnc upgrade --force

# Enable verbose output during upgrade
fnc upgrade --verbose
```

---

# 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

# 🤝 Support & Community

- **Issues**: [GitHub Issues](https://github.com/eulke/fnc/issues)
- **Discussions**: [GitHub Discussions](https://github.com/eulke/fnc/discussions)
- **Documentation**: [Project Wiki](https://github.com/eulke/fnc/wiki)

---

**Built with ❤️ in Rust for the DevOps community**