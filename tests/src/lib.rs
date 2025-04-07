#[cfg(test)]
mod tests {
    // No imports from cli::error needed
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    use version::{Version, VersionType};

    fn create_test_rust_project(dir: &Path) -> std::io::Result<()> {
        let cargo_toml = r#"[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;

        let changelog = r"# Changelog

## [Unreleased]
### Fixed
- Test fix

## [0.1.0] - 2023-01-01
### Added
- Test added
";

        fs::write(dir.join("Cargo.toml"), cargo_toml)?;
        fs::create_dir_all(dir.join("src"))?;
        fs::write(dir.join("src").join("lib.rs"), "// Test file")?;
        fs::write(dir.join("CHANGELOG.md"), changelog)?;
        Ok(())
    }

    fn create_test_js_project(dir: &Path) -> std::io::Result<()> {
        let package_json = r#"{
  "name": "test_project",
  "version": "0.1.0",
  "description": "Test project",
  "main": "index.js",
  "dependencies": {}
}"#;

        let changelog = r"# Changelog

## [Unreleased]
### Fixed
- Test fix

## [0.1.0] - 2023-01-01
### Added
- Test added
";

        fs::write(dir.join("package.json"), package_json)?;
        fs::write(dir.join("index.js"), "// Test file")?;
        fs::write(dir.join("CHANGELOG.md"), changelog)?;
        Ok(())
    }

    #[test]
    fn test_rust_version_increment() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        create_test_rust_project(project_path).unwrap();

        let current_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(current_version.to_string(), "0.1.0");

        let new_version = Version::increment(&current_version, &VersionType::Patch).unwrap();
        assert_eq!(new_version.to_string(), "0.1.1");

        Version::write_to_project(project_path, &new_version).unwrap();

        let updated_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(updated_version.to_string(), "0.1.1");
    }

    #[test]
    fn test_js_version_increment() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        create_test_js_project(project_path).unwrap();

        let current_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(current_version.to_string(), "0.1.0");

        let new_version = Version::increment(&current_version, &VersionType::Minor).unwrap();
        assert_eq!(new_version.to_string(), "0.2.0");

        Version::write_to_project(project_path, &new_version).unwrap();

        let updated_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(updated_version.to_string(), "0.2.0");
    }

    #[test]
    fn test_changelog_update() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        let changelog_path = project_path.join("CHANGELOG.md");

        // Create a basic changelog file first
        let initial_content = r"# Changelog

## [Unreleased]
### Fixed
- Test fix

## [0.1.0] - 2023-01-01
### Added
- Initial release";
        fs::write(&changelog_path, initial_content).unwrap();

        let config = changelog::ChangelogConfig::default();
        // Use GitHub format which matches the expected output format
        let format = changelog::ChangelogFormat::GitHub;

        assert!(changelog_path.exists());

        // Read content and create changelog with string content, not path
        let content = fs::read_to_string(&changelog_path).unwrap();
        let changelog = changelog::Changelog::new(content, config.clone(), format).unwrap();

        // Replace unreleased with a new version
        let new_content = changelog
            .replace_unreleased("0.2.0", "Test User (test@example.com)")
            .unwrap();

        // Write the updated content back to the file
        fs::write(&changelog_path, new_content).unwrap();

        // Create a new changelog instance with the updated content
        let updated_content = fs::read_to_string(&changelog_path).unwrap();
        let updated_changelog = changelog::Changelog::new(updated_content, config, format).unwrap();

        // Now extract changes from the updated changelog
        let changes = updated_changelog.extract_changes(Some("0.2.0")).unwrap();

        assert!(changes.contains("## [0.2.0]"));

        let content = fs::read_to_string(&changelog_path).unwrap();
        assert!(content.contains("0.1.0"));
    }

    #[test]
    fn test_version_and_changelog_integration() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        create_test_rust_project(project_path).unwrap();

        let changelog_path = project_path.join("CHANGELOG.md");
        let config = changelog::ChangelogConfig::default();
        // Use GitHub format which matches the expected output format
        let format = changelog::ChangelogFormat::GitHub;

        // First minor version increment: 0.1.0 -> 0.2.0
        let new_version = Version::update_in_project(project_path, &VersionType::Minor).unwrap();
        assert_eq!(new_version.to_string(), "0.2.0");

        // Read content and update changelog with the new version
        let content = fs::read_to_string(&changelog_path).unwrap();
        let changelog = changelog::Changelog::new(content, config.clone(), format).unwrap();

        // Replace unreleased with a new version
        let new_content = changelog
            .replace_unreleased(&new_version.to_string(), "Test User (test@example.com)")
            .unwrap();

        // Write the updated content back to the file
        fs::write(&changelog_path, new_content).unwrap();

        // Verify the results
        let updated_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(updated_version.to_string(), "0.2.0");

        let content = fs::read_to_string(&changelog_path).unwrap();
        assert!(content.contains("## [0.2.0]"));
    }

    #[test]
    fn test_changelog_add_version_without_unreleased() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        let changelog_path = project_path.join("CHANGELOG.md");

        // Create a changelog without an unreleased section
        let initial_content = r"# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2023-01-01
### Added
- Initial release";
        fs::write(&changelog_path, initial_content).unwrap();

        let config = changelog::ChangelogConfig::default();
        // Use GitHub format which matches the expected output format
        let format = changelog::ChangelogFormat::GitHub;

        // Read the content and create a changelog object
        let content = fs::read_to_string(&changelog_path).unwrap();
        let changelog = changelog::Changelog::new(content, config, format).unwrap();

        // Replace unreleased (which doesn't exist) with a new version
        let new_content = changelog
            .replace_unreleased("0.2.0", "Test User (test@example.com)")
            .unwrap();

        // Write the updated content back to the file
        fs::write(&changelog_path, new_content).unwrap();

        // Verify the content
        let updated_content = fs::read_to_string(&changelog_path).unwrap();

        // Expected content structure after adding new version
        let expected_structure = r"# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2023-01-01

## [0.1.0] - 2023-01-01
### Added
- Initial release";

        // Check that the updated content contains all the expected elements in the right order
        assert!(updated_content.contains("# Changelog"));
        assert!(updated_content.contains("## [0.2.0]"));
        assert!(updated_content.contains("## [0.1.0]"));

        // Verify the structure match (excluding any date differences)
        let normalized_content = updated_content
            .lines()
            .map(|line| {
                if line.starts_with("## [") {
                    // Normalize the date part to 2023-01-01 for comparison
                    let parts: Vec<&str> = line.split(" - ").collect();
                    if parts.len() > 1 {
                        format!("{} - 2023-01-01", parts[0])
                    } else {
                        line.to_string()
                    }
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n");

        // Print contents for debugging
        eprintln!("\nActual content:\n---\n{}\n---\n", updated_content);
        eprintln!("\nNormalized content:\n---\n{}\n---\n", normalized_content);
        eprintln!("\nExpected structure:\n---\n{}\n---\n", expected_structure);

        // Verify the structure matches our expected format
        assert!(
            normalized_content.contains(expected_structure),
            "Generated changelog structure doesn't match expected format"
        );

        // Additional structural checks
        let version_pos = updated_content.find("## [0.2.0]").unwrap();
        let old_version_pos = updated_content.find("## [0.1.0]").unwrap();

        // Verify new version appears before old version
        assert!(
            version_pos < old_version_pos,
            "New version should be placed before old version"
        );
    }
}
