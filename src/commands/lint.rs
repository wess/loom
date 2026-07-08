//! `loom lint` — validate one manifest or the whole repository.

use crate::commands::open_repo;
use crate::config::Config;
use crate::manifest::{Manifest, Severity};
use crate::output;
use anyhow::{Result, bail};
use colored::Colorize;
use std::path::Path;

pub fn run(path: Option<&str>) -> Result<()> {
    match path {
        Some(file) => lint_one(Path::new(file)),
        None => lint_repo(),
    }
}

fn lint_one(path: &Path) -> Result<()> {
    let manifest = Manifest::load(path)?;
    let clean = report(&manifest, &path.display().to_string());
    if !clean {
        bail!("manifest has errors");
    }
    Ok(())
}

fn lint_repo() -> Result<()> {
    let config = Config::load()?;
    let repo = open_repo(&config);
    let (entries, errors) = repo.entries()?;

    let mut failed = 0;
    for entry in &entries {
        if !report(&entry.manifest, &entry.manifest.name) {
            failed += 1;
        }
    }
    for (file, err) in &errors {
        failed += 1;
        output::error(&format!("{}: {}", file.display(), err));
    }

    println!();
    if failed == 0 {
        output::ok(&format!("{} manifests, no errors", entries.len()));
        Ok(())
    } else {
        bail!("{failed} manifest(s) with errors");
    }
}

/// Print a manifest's lint result. Returns true when free of hard errors.
fn report(manifest: &Manifest, label: &str) -> bool {
    let problems = manifest.lint();
    let errors = problems.iter().filter(|p| p.severity == Severity::Error).count();

    if problems.is_empty() {
        println!("{} {}", "✓".green().bold(), label);
        return true;
    }

    let mark = if errors == 0 { "!".yellow().bold() } else { "✗".red().bold() };
    println!("{} {}", mark, label.bold());
    for problem in &problems {
        let tag = match problem.severity {
            Severity::Error => "error".red(),
            Severity::Warning => "warn".yellow(),
        };
        println!("    {} {}", tag, problem.message);
    }
    errors == 0
}
