mod common;
mod go;
mod javascript;
mod rust;

use go::Go;
use javascript::Javascript;
use rust::Rust;
use std::fs;

use crate::ports::PackageOperations;

pub struct Language;
impl Language {
    pub fn detect() -> Option<Box<dyn PackageOperations>> {
        if fs::metadata("Cargo.toml").is_ok() {
            Some(Box::new(Rust {}))
        } else if fs::metadata("package.json").is_ok() {
            Some(Box::new(Javascript {}))
        } else if fs::metadata("go.mod").is_ok() || fs::metadata("pom.xml").is_ok() {
            Some(Box::new(Go {}))
        } else {
            None
        }
    }
}
