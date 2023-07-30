use std::error::Error;
use std::process::Command;
use crate::ports::{AuthorInfo, VCSOperations};
use git2::{AnnotatedCommit, AutotagOption, BranchType, build, Config, FetchOptions, Reference, RemoteCallbacks, Repository, StatusOptions};

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
        // let mut remote = self.repo.find_remote("origin")?;
        // let head = self.repo.head()?;
        // let current_branch = head.shorthand().unwrap();
        // let fetch_commit = do_fetch(&self.repo, &[current_branch], &mut remote)?;
        // do_merge(&self.repo, current_branch, fetch_commit)?;
        //
        // Ok(())

        Command::new("git").arg("pull").output()?;
        Ok(())
    }
}

fn do_fetch<'a>(
    repo: &'a Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
    ) -> Result<AnnotatedCommit<'a>, git2::Error> {
    let cb = RemoteCallbacks::new();

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    // Always fetch all tags.
    // Perform a download and also update tips
    fo.download_tags(AutotagOption::All);
    println!("Fetching {} for repo", remote.name().unwrap());
    remote.fetch(refs, Some(&mut fo), None)?;

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    repo.reference_to_annotated_commit(&fetch_head)
}

fn fast_forward(
    repo: &Repository,
    lb: &mut Reference,
    rc: &AnnotatedCommit,
    ) -> Result<(), git2::Error> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };
    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    println!("{}", msg);
    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(
        build::CheckoutBuilder::default()
            // For some reason the force is required to make the working directory actually get updated
            // I suspect we should be adding some logic to handle dirty working directory states
            // but this is just an example so maybe not.
            .force(),
    ))?;
    Ok(())
}

fn normal_merge(
    repo: &Repository,
    local: &AnnotatedCommit,
    remote: &AnnotatedCommit,
    ) -> Result<(), git2::Error> {
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;
    let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

    if idx.has_conflicts() {
        println!("Merge conflicts detected...");
        repo.checkout_index(Some(&mut idx), None)?;
        return Ok(());
    }
    let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
    // now create the merge commit
    let msg = format!("Merge: {} into {}", remote.id(), local.id());
    let sig = repo.signature()?;
    let local_commit = repo.find_commit(local.id())?;
    let remote_commit = repo.find_commit(remote.id())?;
    // Do our merge commit and set current branch head to that commit.
    let _merge_commit = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &msg,
        &result_tree,
        &[&local_commit, &remote_commit],
    )?;
    // Set working tree to match head.
    repo.checkout_head(None)?;
    Ok(())
}

fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: AnnotatedCommit<'a>,
    ) -> Result<(), git2::Error> {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appropriate merge
    if analysis.0.is_fast_forward() {
        println!("Doing a fast forward");
        // do a fast forward
        let refname = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, &fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        };
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(repo, &head_commit, &fetch_commit)?;
    } else {
        println!("Nothing to do...");
    }
    Ok(())
}
