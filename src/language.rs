mod common;
mod go;
mod javascript;
mod rust;

use go::Go;
use javascript::Javascript;
use rust::Rust;
use std::fs;
use thiserror::Error;

use crate::ports::PackageOperations;

const CARGO_TOML: &str = "Cargo.toml";
const PACKAGE_JSON: &str = "package.json";
const GO_MOD: &str = "go.mod";

#[derive(Error, Debug)]
pub enum LanguageError {
    #[error("Failed to read manifest file: {0}")]
    ManifestReadError(#[from] std::io::Error),
}

pub struct Language;

struct LanguageStrategy {
    file: &'static str,
    operation: Box<dyn PackageOperations>,
}

impl Language {
    fn strategies() -> Vec<LanguageStrategy> {
        vec![
            LanguageStrategy {
                file: CARGO_TOML,
                operation: Box::new(Rust {}),
            },
            LanguageStrategy {
                file: PACKAGE_JSON,
                operation: Box::new(Javascript {}),
            },
            LanguageStrategy {
                file: GO_MOD,
                operation: Box::new(Go {}),
            },
        ]
    }

    pub fn detect() -> Option<Box<dyn PackageOperations>> {
        for strategy in Self::strategies() {
            if fs::metadata(strategy.file).is_ok() {
                return Some(strategy.operation);
            }
        }
        None
    }
}
