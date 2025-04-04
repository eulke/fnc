use colored::Colorize;
use std::io::{self, Write};

/// Print a status message with a spinner-like indicator
pub fn status_message(message: &str) {
    println!("{} {} ... ", "⏳".yellow(), message.bright_white());
    io::stdout().flush().unwrap();
}

/// Print a success message
pub fn success_message(message: &str) {
    println!("{} {}", "✅".green(), message.green());
}

/// Print a warning message
pub fn warning_message(message: &str) {
    println!("{} {}", "⚠️ ".yellow(), message.yellow());
}

/// Print an error message
pub fn error_message(message: &str) {
    eprintln!("{} {}", "❌".red(), message.red().bold());
}

/// Print a section header to separate logical sections of output
pub fn section_header(title: &str) {
    println!("\n{}", format!("==== {} ====", title).cyan().bold());
}

/// Print a simple informational message
pub fn info_message(message: &str) {
    println!("{} {}", "ℹ️ ".blue(), message.blue());
}

/// Print a step in a numbered list of steps
pub fn step_message(step_number: usize, message: &str) {
    println!("  {}. {}", format!("{}", step_number).cyan(), message);
}
