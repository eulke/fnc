use std::{error::Error, process::Command};

use git2::{BranchType, Repository as GitRepository, StatusOptions};

pub trait Repository {
    fn open() -> Self;
    fn validate_status(&self) -> Result<bool, Box<dyn Error>>;
    fn create_branch(&self, name: &str) -> Result<(), Box<dyn Error>>;
    fn checkout_branch(&self, branch_name: &str) -> Result<(), Box<dyn Error>>;
    fn pull(&self) -> Result<(), Box<dyn Error>>;
    fn get_default_branch(&self) -> Result<String, Box<dyn Error>>;
}

pub struct RealGitRepository { repo: GitRepository }
impl Repository for RealGitRepository {
    fn open() -> Self {
        let repo = GitRepository::discover(".").unwrap();
        Self { repo }
    }

    fn validate_status(&self) -> Result<bool, Box<dyn Error>> {
        let repo = &self.repo;

        let mut options = StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut options))?;

        Ok(statuses.is_empty())
    }

    fn create_branch(&self, name: &str) -> Result<(), Box<dyn Error>> {
        let repo = &self.repo;

        let current_commit = repo.head()?.peel_to_commit()?;

        let branch_ref = repo.branch(name, &current_commit, false)?;

        let mut branch = repo.find_branch(name, BranchType::Local)?;

        branch.set_upstream(Some(branch_ref.name()?.unwrap()))?;

        Ok(())
    }

    fn checkout_branch(&self, branch_name: &str) -> Result<(), Box<dyn Error>> {
        let repo = &self.repo;
        let obj = repo.revparse_single(&("refs/heads/".to_owned() + branch_name))?;

        repo.checkout_tree(&obj, None)?;
        repo.set_head(&("refs/heads/".to_owned() + branch_name))?;

        Ok(())
    }

    fn pull(&self) -> Result<(), Box<dyn Error>> {
        Command::new("git").arg("pull").output()?;
        Ok(())
    }

    fn get_default_branch(&self) -> Result<String, Box<dyn Error>> {
        let repo = &self.repo;

        let branch = repo.find_branch("develop", BranchType::Local).unwrap_or_else(|_| {
            repo.find_branch("master", BranchType::Local).unwrap_or_else(|_| {
                repo.find_branch("main", BranchType::Local).unwrap_or_else(|_| {
                    panic!("No default branch found")
                })
            })
        });

        let branch_name = branch.name()?.unwrap().to_string();

        Ok(branch_name)
    }
}

