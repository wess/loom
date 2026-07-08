//! Loom — a package manager for AI skills.

mod cli;
mod commands;
mod config;
mod fetch;
mod generate;
mod install;
mod manifest;
mod output;
mod paths;
mod repo;
mod site;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            output::error(&format!("{err:#}"));
            ExitCode::FAILURE
        }
    }
}
