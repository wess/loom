//! Small, consistent terminal output helpers.

use colored::Colorize;

/// A leading arrow prompt for progress lines, Homebrew style.
pub fn step(message: &str) {
    println!("{} {}", "==>".blue().bold(), message.bold());
}

/// A success line.
pub fn ok(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// A warning line to stderr.
pub fn warn(message: &str) {
    eprintln!("{} {}", "warning:".yellow().bold(), message);
}

/// An error line to stderr.
pub fn error(message: &str) {
    eprintln!("{} {}", "error:".red().bold(), message);
}

/// A muted secondary line.
pub fn detail(label: &str, value: &str) {
    println!("  {} {}", format!("{label}:").dimmed(), value);
}
