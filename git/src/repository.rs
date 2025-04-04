use std::process::Command;

use crate::error::{GitError, Result};
use git2::{BranchType, Repository as GitRepository, StatusOptions};

pub trait Repository {
    fn open() -> Result<Self>
    where
        Self: Sized;
    fn validate_status(&self) -> Result<bool>;
    fn create_branch(&self, name: &str) -> Result<()>;
    fn checkout_branch(&self, branch_name: &str) -> Result<()>;
    fn pull(&self) -> Result<()>;
    fn get_default_branch(&self) -> Result<String>;
    fn get_main_branch(&self) -> Result<String>;
    fn search_in_branch(&self, branch: &str, text: &str) -> Result<bool>;
    fn get_diff_from_main(&self) -> Result<String>;
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
            GitError::RepositoryError(format!("Failed to discover git repository: {}", e))
        })?;
        Ok(Self { repo })
    }

    fn validate_status(&self) -> Result<bool> {
        let repo = &self.repo;

        let mut options = StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut options)).map_err(|e| {
            GitError::RepositoryError(format!("Failed to get repository status: {}", e))
        })?;

        Ok(statuses.is_empty())
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        let repo = &self.repo;

        let current_commit = repo
            .head()
            .map_err(|e| GitError::RepositoryError(format!("Failed to get HEAD: {}", e)))?
            .peel_to_commit()
            .map_err(|e| {
                GitError::RepositoryError(format!("Failed to peel HEAD to commit: {}", e))
            })?;

        let branch_ref = repo.branch(name, &current_commit, false).map_err(|e| {
            GitError::BranchError(format!("Failed to create branch '{}': {}", name, e))
        })?;

        let mut branch = repo.find_branch(name, BranchType::Local).map_err(|e| {
            GitError::BranchNotFound(format!(
                "Failed to find newly created branch '{}': {}",
                name, e
            ))
        })?;

        let branch_name = branch_ref.name()?.ok_or_else(|| {
            GitError::RepositoryError(format!("Failed to get name for branch '{}'", name))
        })?;

        branch.set_upstream(Some(branch_name)).map_err(|e| {
            GitError::RepositoryError(format!(
                "Failed to set upstream for branch '{}': {}",
                name, e
            ))
        })?;

        Ok(())
    }

    fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let repo = &self.repo;
        let branch_ref = format!("refs/heads/{}", branch_name);

        let obj = repo.revparse_single(&branch_ref).map_err(|e| {
            GitError::BranchError(format!("Failed to resolve branch '{}': {}", branch_name, e))
        })?;

        repo.checkout_tree(&obj, None).map_err(|e| {
            GitError::BranchError(format!(
                "Failed to checkout branch '{}': {}",
                branch_name, e
            ))
        })?;

        repo.set_head(&branch_ref).map_err(|e| {
            GitError::BranchError(format!("Failed to set HEAD to '{}': {}", branch_name, e))
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
        // Use git grep to search for the text in the branch
        // Using std::process::Command because git2 doesn't provide a good API for this
        let output = Command::new("git")
            .args(["grep", "-q", "-i", text, branch])
            .output()
            .map_err(|e| {
                GitError::IoError(e).with_context(format!(
                    "Failed to search for '{}' in branch '{}'",
                    text, branch
                ))
            })?;

        // git grep returns 0 if match found, 1 if no match, >1 if error
        if output.status.code().unwrap_or(0) > 1 {
            return Err(GitError::CommandError(format!(
                "Error searching in branch: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(output.status.success())
    }

    fn get_diff_from_main(&self) -> Result<String> {
        let main_branch = self.get_main_branch()?;

        let fetch_output = Command::new("git")
            .args(["fetch", "origin", &main_branch])
            .output()
            .map_err(|e| {
                GitError::IoError(e).with_context(format!("Failed to fetch origin/{}", main_branch))
            })?;

        if !fetch_output.status.success() {
            return Err(GitError::CommandError(format!(
                "Failed to fetch origin/{}: {}",
                main_branch,
                String::from_utf8_lossy(&fetch_output.stderr)
            )));
        }

        let diff = Command::new("git")
            .args(["diff", &format!("origin/{}", main_branch)])
            .output()
            .map_err(|e| {
                GitError::IoError(e)
                    .with_context(format!("Failed to get diff against origin/{}", main_branch))
            })?;

        if !diff.status.success() {
            return Err(GitError::CommandError(format!(
                "Failed to get diff against origin/{}: {}",
                main_branch,
                String::from_utf8_lossy(&diff.stderr)
            )));
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
