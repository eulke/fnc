use clap::{Parser, Subcommand};
use git2::{BranchType, Repository};
use serde_json::Value;
use std::fs;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    New {
        name: String,
        #[arg(short, long)]
        version: Option<String>,
    },
}

fn handle_new(name: String, version: Option<String>) {
    let branch_name = match name.as_str() {
        "release" => {
            let version = version.unwrap_or_else(|| {
                let package_json =
                    fs::read_to_string("package.json").expect("Failed to read package.json");
                let json: Value =
                    serde_json::from_str(&package_json).expect("Failed to parse package.json");
                let current_version = json["version"]
                    .as_str()
                    .expect("Failed to get version from package.json");
                increment_version(current_version, "patch")
            });
            format!("release/{}", version)
        }
        "hotfix" => format!("hotfix/{}", version.unwrap_or_default()),
        _ => panic!("Invalid name. Only 'release' and 'hotfix' are allowed."),
    };

    create_branch(&branch_name);
}

fn increment_version(version: &str, level: &str) -> String {
    let mut parts: Vec<u32> = version
        .split('.')
        .map(|part| part.parse::<u32>().expect("Failed to parse version part"))
        .collect();

    match level {
        "major" => parts[0] += 1,
        "minor" => parts[1] += 1,
        "patch" => parts[2] += 1,
        _ => panic!("Invalid version level. Only 'major', 'minor', and 'patch' are allowed."),
    }

    format!("{}.{}.{}", parts[0], parts[1], parts[2])
}

fn create_branch(branch_name: &str) {
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

fn get_current_repository() -> Repository {
    let repo = Repository::discover(".").expect("Failed to discover current repository");
    println!("Opened repository: {}", repo.path().display());
    repo
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, version } => {
            handle_new(name, version);
        }
    }
}
