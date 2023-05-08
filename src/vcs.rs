use std::error::Error;

use crate::ports::{AuthorInfo, VCSOperations};
use git2::{BranchType, Config, Repository};

pub struct GitAdapter;

impl VCSOperations for GitAdapter {
    fn create_branch(&self, branch_name: &str) {
        let repo = get_current_repository();

        let head = repo.head().expect("Failed to get head reference");
        let head_commit = head
            .peel_to_commit()
            .expect("Failed to peel head reference to commit");

        let branch_ref = repo
            .branch(branch_name, &head_commit, false)
            .expect("Failed to create branch reference");

        let mut branch = repo
            .find_branch(branch_name, BranchType::Local)
            .expect("Failed to find branch");

        branch
            .set_upstream(Some(branch_ref.name().unwrap().unwrap()))
            .expect("Failed to set upstream");

        println!("Created branch: {}", branch_name);
    }

    fn checkout_branch(&self, branch_name: &str) -> Result<(), Box<dyn Error>> {
        let repo = get_current_repository();
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
}

fn get_current_repository() -> Repository {
    let repo = Repository::discover(".").expect("Failed to discover current repository");
    println!("Opened repository: {}", repo.path().display());
    repo
}
