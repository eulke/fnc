use std::error::Error;
use std::process::Command;
use crate::ports::{AuthorInfo, VCSOperations};
use git2::{BranchType, Config, Repository, StatusOptions};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Failed to discover git repository: {0}")]
    RepositoryNotFound(#[from] git2::Error),
    
    #[error("Git command failed: {0}")]
    CommandFailed(String),
}

pub struct Adapter {
    pub repo: Repository,
}

impl VCSOperations for Adapter {
    fn new() -> Self {
        match Repository::discover(".") {
            Ok(repo) => Self { repo },
            Err(e) => panic!("Failed to initialize git repository: {}", e), // We'll keep the panic here as this is a CLI tool
        }
    }

    fn create_branch(&self, branch_name: &str) -> Result<(), Box<dyn Error>> {
        let repo = &self.repo;
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;
        let branch_ref = repo.branch(branch_name, &head_commit, false)?;
        let mut branch = repo.find_branch(branch_name, BranchType::Local)?;
        branch.set_upstream(Some(branch_ref.name()?.ok_or("Invalid branch name")?))
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    fn checkout_branch(&self, branch_name: &str) -> Result<(), Box<dyn Error>> {
        let repo = &self.repo;
        let obj = repo.revparse_single(&("refs/heads/".to_owned() + branch_name))?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head(&("refs/heads/".to_owned() + branch_name))?;
        Ok(())
    }

    fn read_config(&self) -> Result<AuthorInfo, Box<dyn Error>> {
        let config = Config::open_default()?;
        let name = config.get_string("user.name")?;
        let email = config.get_string("user.email")?;
        Ok(AuthorInfo { name, email })
    }

    fn validate_status(&self) -> Result<bool, Box<dyn Error>> {
        let repo = &self.repo;
        let mut options = StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);
        let statuses = repo.statuses(Some(&mut options))?;
        Ok(statuses.is_empty())
    }

    fn get_default_branch(&self) -> Result<String, Box<dyn Error>> {
        let repo = &self.repo;
        
        // Try branches in order: develop, main, master
        for branch_name in ["develop", "main", "master"] {
            if let Ok(branch) = repo.find_branch(branch_name, BranchType::Local) {
                if let Ok(Some(name)) = branch.name() {
                    return Ok(name.to_string());
                }
            }
        }
        
        Err("No default branch found".into())
    }

    fn pull(&self) -> Result<(), Box<dyn Error>> {
        Command::new("git")
            .arg("pull")
            .output()
            .map_err(|e| Box::new(GitError::CommandFailed(e.to_string())) as Box<dyn Error>)?;
        Ok(())
    }
}
