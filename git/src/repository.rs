use std::process::Command;

use git2::{BranchType, Repository as GitRepository, StatusOptions};
use crate::error::{GitError, Result};

pub trait Repository {
    fn open() -> Result<Self> where Self: Sized;
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

pub struct RealGitRepository { repo: GitRepository }
impl RealGitRepository {
    // Helper method to find a branch from a list of possible branch names
    fn find_branch_from_candidates(&self, branch_names: &[&str], error_msg: &str) -> Result<String> {
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
        let repo = GitRepository::discover(".")?;
        Ok(Self { repo })
    }

    fn validate_status(&self) -> Result<bool> {
        let repo = &self.repo;

        let mut options = StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut options))?;

        Ok(statuses.is_empty())
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        let repo = &self.repo;

        let current_commit = repo.head()?.peel_to_commit()?;

        let branch_ref = repo.branch(name, &current_commit, false)?;

        let mut branch = repo.find_branch(name, BranchType::Local)?;

        branch.set_upstream(Some(branch_ref.name()?.unwrap()))?;

        Ok(())
    }

    fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let repo = &self.repo;
        let obj = repo.revparse_single(&("refs/heads/".to_owned() + branch_name))?;

        repo.checkout_tree(&obj, None)?;
        repo.set_head(&("refs/heads/".to_owned() + branch_name))?;

        Ok(())
    }

    fn pull(&self) -> Result<()> {
        let output = Command::new("git")
            .arg("pull")
            .output()
            .map_err(|e| GitError::IoError(e).with_context("Failed to execute git pull command"))?;
        
        if !output.status.success() {
            return Err(GitError::CommandError(String::from_utf8_lossy(&output.stderr).to_string()));
        }
        
        Ok(())
    }

    fn get_default_branch(&self) -> Result<String> {
        self.find_branch_from_candidates(
            &["develop", "master", "main"],
            "No default branch found"
        )
    }

    fn get_main_branch(&self) -> Result<String> {
        self.find_branch_from_candidates(
            &["main", "master"],
            "No main branch found"
        )
    }
    
    fn search_in_branch(&self, branch: &str, text: &str) -> Result<bool> {
        // Use git grep to search for the text in the branch
        // Using std::process::Command because git2 doesn't provide a good API for this
        let output = Command::new("git")
            .args(["grep", "-q", "-i", text, branch])
            .output()
            .map_err(|e| GitError::IoError(e).with_context(format!("Failed to search for '{}' in branch '{}'", text, branch)))?;
        
        // git grep returns 0 if match found, 1 if no match, >1 if error
        if output.status.code().unwrap_or(0) > 1 {
            return Err(GitError::CommandError(format!("Error searching in branch: {}", String::from_utf8_lossy(&output.stderr))));
        }
        
        Ok(output.status.success())
    }

    fn get_diff_from_main(&self) -> Result<String> {
        let main_branch = self.get_main_branch()?;
        
        let fetch_output = Command::new("git")
            .args(["fetch", "origin", &main_branch])
            .output()
            .map_err(|e| GitError::IoError(e).with_context(format!("Failed to fetch origin/{}", main_branch)))?;
            
        if !fetch_output.status.success() {
            return Err(GitError::CommandError(format!("Failed to fetch origin/{}: {}", 
                main_branch, String::from_utf8_lossy(&fetch_output.stderr))));
        }
        
        let diff = Command::new("git")
            .args(["diff", &format!("origin/{}", main_branch)])
            .output()
            .map_err(|e| GitError::IoError(e).with_context(format!("Failed to get diff against origin/{}", main_branch)))?;

        if !diff.status.success() {
            return Err(GitError::CommandError(format!("Failed to get diff against origin/{}: {}", 
                main_branch, String::from_utf8_lossy(&diff.stderr))));
        }

        Ok(String::from_utf8_lossy(&diff.stdout).to_string())
    }

    fn get_current_branch(&self) -> Result<String> {
        let repo = &self.repo;
        
        let head = repo.head()?;
        if !head.is_branch() {
            return Err(GitError::RepositoryError("HEAD is not a branch".to_string()));
        }
        
        let branch_name = head.shorthand()
            .ok_or_else(|| GitError::RepositoryError("Invalid branch name".to_string()))?
            .to_string();
            
        Ok(branch_name)
    }
}

