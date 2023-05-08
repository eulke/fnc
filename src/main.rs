mod cli;
mod package;
mod ports;
mod semver;
mod vcs;

use clap::Parser;
use cli::{Cli, Commands};
use ports::{AuthorInfo, PackageOperations, VCSOperations};

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

    package.increment_version(&incremented_semver, &language, &author);
}

fn main() {
    let cli = Cli::parse();
    let vcs = vcs::GitAdapter;
    let package = package::Adapter;

    match cli.command {
        Commands::New { name, version } => {
            handle_new(&vcs, &package, name, version);
        }
    }
}
