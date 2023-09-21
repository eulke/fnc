use std::error::Error;
use std::process::Command;
use crate::ports::{AuthorInfo, VCSOperations};
use git2::{ BranchType, Config, Repository, StatusOptions};

pub struct Adapter {
    pub repo: Repository,
}

impl VCSOperations for Adapter {
    fn new() -> Self {
        let repo = Repository::discover(".").unwrap();
        Self { repo }
    }

    fn create_branch(&self, branch_name: &str) -> Result<(), Box<dyn Error>> {
        let repo = &self.repo;

        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;

        let branch_ref = repo.branch(branch_name, &head_commit, false)?;

        let mut branch = repo.find_branch(branch_name, BranchType::Local)?;

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

    fn pull(&self) -> Result<(), Box<dyn Error>> {
        Command::new("git").arg("pull").output()?;
        Ok(())
    }
}
