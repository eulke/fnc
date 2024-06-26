mod cli;
mod git;
mod language;
mod ports;
mod semver;

use std::process;

use clap::Parser;
use cli::{Cli, Commands};
use language::Language;
use ports::{AuthorInfo, PackageOperations, VCSOperations};
use crate::cli::{DeployType, Version};

fn handle_deploy<T: VCSOperations>(
    vcs: &T,
    language: &dyn PackageOperations,
    deploy_type: DeployType,
    version: Version,
) {

    match vcs.validate_status() {
        Ok(true) => println!("Status is clean. Process continues"),
        Ok(false) => {
            println!("There are uncommitted changes. Process finished");
            process::exit(1);
        }
        Err(_) => {
            println!("Error validating status. Process finished");
            process::exit(1);
        }
    }

    match deploy_type {
        DeployType::Release => {
            let default_branch = match vcs.get_default_branch() {
                Ok(branch) => branch,
                Err(_) => {
                    println!("Error getting default branch. Process finished");
                    process::exit(1);
                }
            };

            println!("Checking out to the default branch: {}", default_branch);
            vcs.checkout_branch(&default_branch).unwrap_or_else(|_| {
                println!("Cannot checkout to the default branch.");
                process::exit(1);
            });
        },
        DeployType::Hotfix => {
            println!("Checking out to the master or main branch");
            vcs.checkout_branch("master").unwrap_or_else(|_| {
                vcs.checkout_branch("main").unwrap_or_else(|_| {
                    println!("Cannot checkout to the master or main branch.");
                    process::exit(1);
                });
            });
        },
    }

    println!("Pulling from remote");
    vcs.pull().unwrap_or_else(|_| {
        println!("Error pulling from remote. Process finished");
        process::exit(1);
    });

    let current_semver = language.current_pkg_version();
    let incremented_semver = semver::increment(&current_semver, &version);
    println!("Incrementing version from {} to {}", &current_semver, &incremented_semver);
    let branch_name = match deploy_type {
        DeployType::Release => format!("release/{}", &incremented_semver),
        DeployType::Hotfix => format!("hotfix/{}", &incremented_semver),
    };

    println!("Creating branch {}", &branch_name);
    vcs.create_branch(&branch_name).unwrap_or_else(|_| {
        println!("Error crating the branch. Process finished");
        process::exit(1);
    });

    println!("Checking out to the newly created branch");
    vcs.checkout_branch(&branch_name).unwrap_or_else(|_| {
        println!("Cannot checkout to the newly created branch.");
        process::exit(1);
    });

    let author = vcs.read_config().unwrap_or_else(|_| {
        println!("Failed to get author info, add manually");
        AuthorInfo {
            name: String::from(""),
            email: String::from(""),
        }
    });

    println!("Writing version");
    match language.increment_pkg_version(&incremented_semver, &author) {
        Ok(_) => println!("Version incremented successfully."),
        Err(_) => {
            println!("Error encountered incrementing version");
            process::exit(1);
        }
    }

    println!("All done!");
}

fn main() {
    let cli = Cli::parse();

    let vcs = git::Adapter::new();

    let language = match Language::detect() {
        Some(lang) => lang,
        None => panic!("Unable to detect language"),
    };

    match cli.command {
        Commands::Deploy { deploy_type, version } => {
            handle_deploy(&vcs, &(*language), deploy_type, version);
        }
    }
}
