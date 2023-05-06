mod cli;
mod package;
mod ports;
mod semver;
mod vcs;

use clap::Parser;
use cli::{Cli, Commands};
use ports::{PackageOperations, VCSOperations};

fn handle_new<T: VCSOperations, U: PackageOperations>(
    vcs: &T,
    package: &U,
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

    let language = match semver::detect_language() {
        Some(lang) => lang,
        None => panic!("Unable to detect language"),
    };
    let current_semver = semver::get_current(&language);
    let incremented_semver = semver::increment(&current_semver, &version);
    let branch_name = match name.as_str() {
        "release" => format!("release/{}", &incremented_semver),
        "hotfix" => format!("hotfix/{}", &incremented_semver),
        _ => panic!("Invalid name. Only 'release' and 'hotfix' are allowed."),
    };

    vcs.create_branch(&branch_name);
    vcs.checkout_branch(&branch_name).unwrap();
    package.increment_version(&incremented_semver, &language);
}

fn main() {
    let cli = Cli::parse();
    let vcs = vcs::Adapter;
    let package = package::Adapter;

    match cli.command {
        Commands::New { name, version } => {
            handle_new(&vcs, &package, name, version);
        }
    }
}
