#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    use version::{Version, VersionType, SemverVersion};
    use changelog;

    fn create_test_rust_project(dir: &Path) -> std::io::Result<()> {
        let cargo_toml = r#"[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
        fs::write(dir.join("Cargo.toml"), cargo_toml)?;
        fs::create_dir_all(dir.join("src"))?;
        fs::write(dir.join("src").join("lib.rs"), "// Test file")?;
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
        fs::write(dir.join("package.json"), package_json)?;
        fs::write(dir.join("index.js"), "// Test file")?;
        Ok(())
    }

    #[test]
    fn test_rust_version_increment() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        
        // Create a test Rust project
        create_test_rust_project(project_path).unwrap();

        // Read the current version
        let current_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(current_version.to_string(), "0.1.0");

        // Increment the version (patch)
        let new_version = Version::increment(&current_version, &VersionType::Patch).unwrap();
        assert_eq!(new_version.to_string(), "0.1.1");

        // Write the new version back
        Version::write_to_project(project_path, &new_version).unwrap();

        // Read the version again to confirm it was updated
        let updated_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(updated_version.to_string(), "0.1.1");
    }

    #[test]
    fn test_js_version_increment() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        
        // Create a test JS project
        create_test_js_project(project_path).unwrap();

        // Read the current version
        let current_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(current_version.to_string(), "0.1.0");

        // Increment the version (minor)
        let new_version = Version::increment(&current_version, &VersionType::Minor).unwrap();
        assert_eq!(new_version.to_string(), "0.2.0");

        // Write the new version back
        Version::write_to_project(project_path, &new_version).unwrap();

        // Read the version again to confirm it was updated
        let updated_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(updated_version.to_string(), "0.2.0");
    }

    #[test]
    fn test_changelog_update() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        let changelog_path = project_path.join("CHANGELOG.md");
        
        // Create initial changelog
        changelog::ensure_changelog_exists(&changelog_path, "0.1.0", "Test User (test@example.com)").unwrap();
        
        // Verify it exists
        assert!(changelog_path.exists());
        
        // Update it with a new version
        changelog::update_changelog(&changelog_path, "0.2.0", "Test User (test@example.com)").unwrap();
        
        // Extract changes for the new version
        let changes = changelog::extract_changes(&changelog_path, Some("0.2.0")).unwrap();
        
        // Verify the changes include the new version
        assert!(changes.contains("## [0.2.0]"));
        
        // Initial changelog should still have 0.1.0 version
        let content = fs::read_to_string(&changelog_path).unwrap();
        assert!(content.contains("0.1.0"));
    }

    #[test]
    fn test_version_and_changelog_integration() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        
        // Create a test Rust project
        create_test_rust_project(project_path).unwrap();
        
        // Create a changelog
        let changelog_path = project_path.join("CHANGELOG.md");
        changelog::ensure_changelog_exists(&changelog_path, "0.1.0", "Test User (test@example.com)").unwrap();
        
        // Increment version in project
        let current_version = Version::read_from_project(project_path).unwrap();
        let new_version = Version::update_in_project(project_path, &VersionType::Minor).unwrap();
        assert_eq!(new_version.to_string(), "0.2.0");
        
        // Update changelog with new version
        changelog::update_changelog(&changelog_path, &new_version.to_string(), "Test User (test@example.com)").unwrap();
        
        // Verify everything is updated correctly
        let updated_version = Version::read_from_project(project_path).unwrap();
        assert_eq!(updated_version.to_string(), "0.2.0");
        
        let content = fs::read_to_string(&changelog_path).unwrap();
        assert!(content.contains("## [0.2.0]"));
    }
}