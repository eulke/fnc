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
        let format = changelog::ChangelogFormat::default();

        assert!(changelog_path.exists());

        let mut changelog =
            changelog::Changelog::new(&changelog_path, config.clone(), format).unwrap();
        changelog
            .update_with_version("0.2.0", "Test User (test@example.com)")
            .unwrap();

        let changes = changelog.extract_changes(Some("0.2.0")).unwrap();

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
        let format = changelog::ChangelogFormat::default();

        // First minor version increment: 0.1.0 -> 0.2.0
        let new_version = Version::update_in_project(project_path, &VersionType::Minor).unwrap();
        assert_eq!(new_version.to_string(), "0.2.0");

        // Update changelog with the new version
        let mut changelog =
            changelog::Changelog::new(&changelog_path, config.clone(), format).unwrap();
        changelog
            .update_with_version(&new_version.to_string(), "Test User (test@example.com)")
            .unwrap();

        // Verify the results
        let updated_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(updated_version.to_string(), "0.2.0");

        let content = fs::read_to_string(&changelog_path).unwrap();
        assert!(content.contains("## [0.2.0]"));
    }
}
