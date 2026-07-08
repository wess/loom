//! `loom doctor` — sanity check the environment.

use crate::commands::open_repo;
use crate::config::Config;
use crate::paths;
use anyhow::Result;
use colored::Colorize;
use std::process::Command;

pub fn run() -> Result<()> {
    let mut problems = 0;

    check("git available", git_available(), &mut problems);
    check("loom home writable", home_writable(), &mut problems);

    let config = Config::load()?;
    check(
        "config loads",
        Ok(format!("{} agents configured", config.agents.len())),
        &mut problems,
    );

    let repo = open_repo(&config);
    let repo_state = match repo.entries() {
        Ok((entries, errors)) if errors.is_empty() => {
            Ok(format!("{} manifests", entries.len()))
        }
        Ok((entries, errors)) => Err(format!(
            "{} ok, {} broken manifest(s)",
            entries.len(),
            errors.len()
        )),
        Err(e) => Err(format!("{e:#}")),
    };
    check(
        &format!("repo at {}", repo.dir().display()),
        repo_state,
        &mut problems,
    );

    println!();
    if problems == 0 {
        println!("{} everything looks good", "✓".green().bold());
    } else {
        println!("{} {problems} issue(s) found", "!".yellow().bold());
    }
    Ok(())
}

fn check(label: &str, result: Result<String, String>, problems: &mut u32) {
    match result {
        Ok(detail) => println!("{} {label} — {}", "✓".green().bold(), detail.dimmed()),
        Err(detail) => {
            *problems += 1;
            println!("{} {label} — {}", "✗".red().bold(), detail.red());
        }
    }
}

fn git_available() -> Result<String, String> {
    let output = Command::new("git")
        .arg("--version")
        .output()
        .map_err(|_| "git not found on PATH".to_string())?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn home_writable() -> Result<String, String> {
    let home = paths::home().map_err(|e| e.to_string())?;
    paths::ensure_dir(&home).map_err(|e| e.to_string())?;
    Ok(home.display().to_string())
}
