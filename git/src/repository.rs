use std::process::Command;

use crate::error::{GitError, Result};
use git2::{BranchType, Repository as GitRepository, StatusOptions};

pub trait Repository {
    /// Opens a git repository
    ///
    /// # Errors
    ///
    /// Returns an error if the repository cannot be found or opened
    fn open() -> Result<Self>
    where
        Self: Sized;
        
    /// Validates the status of the repository, checking for uncommitted changes
    ///
    /// # Errors
    ///
    /// Returns an error if the repository status cannot be determined
    fn validate_status(&self) -> Result<bool>;
    
    /// Creates a new branch in the repository
    ///
    /// # Errors
    ///
    /// Returns an error if branch creation fails
    fn create_branch(&self, name: &str) -> Result<()>;
    
    /// Checks out the specified branch
    ///
    /// # Errors
    ///
    /// Returns an error if the checkout operation fails
    fn checkout_branch(&self, branch_name: &str) -> Result<()>;
    
    /// Pulls the latest changes from the remote
    ///
    /// # Errors
    ///
    /// Returns an error if the pull operation fails
    fn pull(&self) -> Result<()>;
    
    /// Gets the default branch of the repository
    ///
    /// # Errors
    ///
    /// Returns an error if the default branch cannot be determined
    fn get_default_branch(&self) -> Result<String>;
    
    /// Gets the main branch of the repository
    ///
    /// # Errors
    ///
    /// Returns an error if the main branch cannot be determined
    fn get_main_branch(&self) -> Result<String>;
    
    /// Searches for text in a specific branch
    ///
    /// # Errors
    ///
    /// Returns an error if the search operation fails
    fn search_in_branch(&self, branch: &str, text: &str) -> Result<bool>;
    
    /// Gets the diff between the current branch and the main branch
    ///
    /// # Errors
    ///
    /// Returns an error if the diff operation fails
    fn get_diff_from_main(&self) -> Result<String>;
    
    /// Gets the name of the current branch
    ///
    /// # Errors
    ///
    /// Returns an error if the current branch cannot be determined
    fn get_current_branch(&self) -> Result<String>;
}

pub struct RealGitRepository {
    repo: GitRepository,
}
impl RealGitRepository {
    // Helper method to find a branch from a list of possible branch names
    fn find_branch_from_candidates(
        &self,
        branch_names: &[&str],
        error_msg: &str,
    ) -> Result<String> {
        let repo = &self.repo;

        for &name in branch_names {
            if let Ok(branch) = repo.find_branch(name, BranchType::Local) {
                if let Ok(Some(branch_name)) = branch.name() {
                    return Ok(branch_name.to_string());
                }
            }
        }

        Err(GitError::BranchNotFound(error_msg.to_string()))
    }
}

impl Repository for RealGitRepository {
    fn open() -> Result<Self> {
        let repo = GitRepository::discover(".").map_err(|e| {
            GitError::RepositoryError(format!("Failed to discover git repository: {e}"))
        })?;
        Ok(Self { repo })
    }

    fn validate_status(&self) -> Result<bool> {
        let repo = &self.repo;

        let mut options = StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut options)).map_err(|e| {
            GitError::RepositoryError(format!("Failed to get repository status: {e}"))
        })?;

        Ok(statuses.is_empty())
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        let repo = &self.repo;

        let current_commit = repo
            .head()
            .map_err(|e| GitError::RepositoryError(format!("Failed to get HEAD: {e}")))?
            .peel_to_commit()
            .map_err(|e| {
                GitError::RepositoryError(format!("Failed to peel HEAD to commit: {e}"))
            })?;

        let branch_ref = repo.branch(name, &current_commit, false).map_err(|e| {
            GitError::BranchError(format!("Failed to create branch '{name}': {e}"))
        })?;

        let mut branch = repo.find_branch(name, BranchType::Local).map_err(|e| {
            GitError::BranchNotFound(format!(
                "Failed to find newly created branch '{name}': {e}"
            ))
        })?;

        let branch_name = branch_ref.name()?.ok_or_else(|| {
            GitError::RepositoryError(format!("Failed to get name for branch '{name}'"))
        })?;

        branch.set_upstream(Some(branch_name)).map_err(|e| {
            GitError::RepositoryError(format!(
                "Failed to set upstream for branch '{name}': {e}"
            ))
        })?;

        Ok(())
    }

    fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let repo = &self.repo;
        let branch_ref = format!("refs/heads/{branch_name}");

        let obj = repo.revparse_single(&branch_ref).map_err(|e| {
            GitError::BranchError(format!("Failed to resolve branch '{branch_name}': {e}"))
        })?;

        repo.checkout_tree(&obj, None).map_err(|e| {
            GitError::BranchError(format!("Failed to checkout branch '{branch_name}': {e}"))
        })?;

        repo.set_head(&branch_ref).map_err(|e| {
            GitError::BranchError(format!("Failed to set HEAD to '{branch_name}': {e}"))
        })?;

        Ok(())
    }

    fn pull(&self) -> Result<()> {
        let output = Command::new("git")
            .arg("pull")
            .output()
            .map_err(|e| GitError::IoError(e).with_context("Failed to execute git pull command"))?;

        if !output.status.success() {
            return Err(GitError::CommandError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    fn get_default_branch(&self) -> Result<String> {
        self.find_branch_from_candidates(&["develop", "master", "main"], "No default branch found")
    }

    fn get_main_branch(&self) -> Result<String> {
        self.find_branch_from_candidates(&["main", "master"], "No main branch found")
    }

    fn search_in_branch(&self, branch: &str, text: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(["grep", "-q", "-i", text, branch])
            .output()
            .map_err(|e| {
                GitError::IoError(e).with_context(format!(
                    "Failed to search for '{text}' in branch '{branch}'"
                ))
            })?;

        if output.status.code().unwrap_or(0) > 1 {
            return Err(GitError::CommandError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(output.status.success())
    }

    fn get_diff_from_main(&self) -> Result<String> {
        let main_branch = self.get_main_branch()?;
        let origin_branch = format!("origin/{main_branch}");

        let fetch_output = Command::new("git")
            .args(["fetch", "origin", &main_branch])
            .output()
            .map_err(|e| {
                GitError::IoError(e).with_context(format!("Failed to fetch {origin_branch}"))
            })?;

        if !fetch_output.status.success() {
            return Err(GitError::CommandError(
                String::from_utf8_lossy(&fetch_output.stderr).to_string(),
            ));
        }

        let diff = Command::new("git")
            .args(["diff", &origin_branch])
            .output()
            .map_err(|e| {
                GitError::IoError(e)
                    .with_context(format!("Failed to get diff against {origin_branch}"))
            })?;

        if !diff.status.success() {
            return Err(GitError::CommandError(
                String::from_utf8_lossy(&diff.stderr).to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&diff.stdout).to_string())
    }

    fn get_current_branch(&self) -> Result<String> {
        let repo = &self.repo;

        let head = repo.head()?;
        if !head.is_branch() {
            return Err(GitError::RepositoryError(
                "HEAD is not a branch".to_string(),
            ));
        }

        let branch_name = head
            .shorthand()
            .ok_or_else(|| GitError::RepositoryError("Invalid branch name".to_string()))?
            .to_string();

        Ok(branch_name)
    }
}
