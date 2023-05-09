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

fn handle_new<T: VCSOperations>(
    vcs: &T,
    language: &dyn PackageOperations,
    name: String,
    version: Option<String>,
) {
    let version = match version {
        Some(version) => version,
        None => {
            println!("No version provided. Incrementing patch version.");
            "patch".to_owned()
        }
    };

    let current_semver = language.current_pkg_version();
    let incremented_semver = semver::increment(&current_semver, &version);
    let branch_name = match name.as_str() {
        "release" => format!("release/{}", &incremented_semver),
        "hotfix" => format!("hotfix/{}", &incremented_semver),
        _ => panic!("Invalid name. Only 'release' and 'hotfix' are allowed."),
    };

    vcs.create_branch(&branch_name).unwrap_or_else(|_| {
        println!("Error crating the branch. Process finished");
        process::exit(1);
    });

    vcs.checkout_branch(&branch_name).unwrap_or_else(|_| {
        println!("Cannot checkout to the newly created branch.");
        process::exit(1);
    });

    let author = match vcs.read_config() {
        Ok(author) => author,
        Err(_) => {
            println!("Failed to get author info, add manually");
            AuthorInfo {
                name: String::from(""),
                email: String::from(""),
            }
        }
    };

    match language.increment_pkg_version(&incremented_semver, &author) {
        Ok(_) => println!("Version incremented successfully."),
        Err(_) => {
            println!("Error encountered incrementing version");
            process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let vcs = git::Adapter;

    let language = match Language::detect() {
        Some(lang) => lang,
        None => panic!("Unable to detect language"),
    };

    match cli.command {
        Commands::New { name, version } => {
            handle_new(&vcs, &(*language), name, version);
        }
    }
}
