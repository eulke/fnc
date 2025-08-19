use crate::error::{HttpDiffError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum OutputCategory {
    Reports,
    Scripts,
    Cache,
    Logs,
}

impl OutputCategory {
    fn subdirectory(self) -> &'static str {
        match self {
            Self::Reports => "reports",
            Self::Scripts => "scripts",
            Self::Cache => "cache",
            Self::Logs => "logs",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutputManager {
    base_dir: PathBuf,
}

impl OutputManager {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    pub fn current_dir() -> Result<Self> {
        let current = std::env::current_dir().map_err(|e| {
            HttpDiffError::general(format!("Failed to get current directory: {}", e))
        })?;
        Ok(Self::new(current))
    }

    pub fn ensure_structure(&self) -> Result<()> {
        let http_diff_dir = self.base_dir.join(".http-diff");

        for category in [
            OutputCategory::Reports,
            OutputCategory::Scripts,
            OutputCategory::Cache,
            OutputCategory::Logs,
        ] {
            let dir_path = http_diff_dir.join(category.subdirectory());
            fs::create_dir_all(&dir_path).map_err(|e| {
                HttpDiffError::general(format!(
                    "Failed to create directory {}: {}",
                    dir_path.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }

    pub fn resolve_output_path<P: AsRef<Path>>(
        &self,
        user_path: P,
        category: OutputCategory,
    ) -> PathBuf {
        let path = user_path.as_ref();

        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir
                .join(".http-diff")
                .join(category.subdirectory())
                .join(path)
        }
    }

    pub fn generate_timestamped_filename(
        &self,
        prefix: &str,
        extension: &str,
        category: OutputCategory,
    ) -> Result<PathBuf> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| HttpDiffError::general(format!("Failed to get system time: {}", e)))?
            .as_secs();

        let filename = format!("{}-{}.{}", prefix, timestamp, extension);
        Ok(self.resolve_output_path(&filename, category))
    }

    pub fn write_file_atomic<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        path: P,
        content: C,
    ) -> Result<()> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                HttpDiffError::general(format!(
                    "Failed to create parent directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        let temp_path = path.with_extension(format!(
            "{}.tmp",
            path.extension().and_then(|s| s.to_str()).unwrap_or("temp")
        ));

        fs::write(&temp_path, content).map_err(|e| {
            HttpDiffError::general(format!(
                "Failed to write temp file {}: {}",
                temp_path.display(),
                e
            ))
        })?;

        fs::rename(&temp_path, path).map_err(|e| {
            HttpDiffError::general(format!(
                "Failed to rename temp file to {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(())
    }

    pub fn category_path(&self, category: OutputCategory) -> PathBuf {
        self.base_dir
            .join(".http-diff")
            .join(category.subdirectory())
    }

    pub fn get_http_diff_root(&self) -> PathBuf {
        self.base_dir.join(".http-diff")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_output_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());

        assert_eq!(manager.base_dir, temp_dir.path());
    }

    #[test]
    fn test_current_dir_creation() {
        let manager = OutputManager::current_dir().unwrap();
        assert!(manager.base_dir.is_absolute());
    }

    #[test]
    fn test_ensure_structure_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());

        manager.ensure_structure().unwrap();

        let http_diff_dir = temp_dir.path().join(".http-diff");
        assert!(http_diff_dir.exists());
        assert!(http_diff_dir.join("reports").exists());
        assert!(http_diff_dir.join("scripts").exists());
        assert!(http_diff_dir.join("cache").exists());
        assert!(http_diff_dir.join("logs").exists());
    }

    #[test]
    fn test_resolve_output_path_relative() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());

        let path = manager.resolve_output_path("test.html", OutputCategory::Reports);
        let expected = temp_dir
            .path()
            .join(".http-diff")
            .join("reports")
            .join("test.html");

        assert_eq!(path, expected);
    }

    #[test]
    fn test_resolve_output_path_absolute() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());
        let absolute_path = temp_dir.path().join("absolute_file.html");

        let path = manager.resolve_output_path(&absolute_path, OutputCategory::Reports);

        assert_eq!(path, absolute_path);
    }

    #[test]
    fn test_generate_timestamped_filename() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());

        let path = manager
            .generate_timestamped_filename("test", "html", OutputCategory::Reports)
            .unwrap();

        assert!(path.to_string_lossy().contains("test-"));
        assert!(path.to_string_lossy().ends_with(".html"));
        assert!(path.starts_with(temp_dir.path().join(".http-diff").join("reports")));
    }

    #[test]
    fn test_write_file_atomic() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());
        manager.ensure_structure().unwrap();

        let file_path = temp_dir
            .path()
            .join(".http-diff")
            .join("reports")
            .join("test.txt");
        let content = b"test content";

        manager.write_file_atomic(&file_path, content).unwrap();

        assert!(file_path.exists());
        let read_content = fs::read(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_category_path() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());

        let reports_path = manager.category_path(OutputCategory::Reports);
        let expected = temp_dir.path().join(".http-diff").join("reports");

        assert_eq!(reports_path, expected);
    }
}
