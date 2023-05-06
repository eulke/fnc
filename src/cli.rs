use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "meli")]
#[command(
    author,
    version,
    about = "Meli CLI tool that automate repetitive tasks"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    New {
        name: String,
        #[arg(short, long)]
        version: Option<String>,
    },
}
