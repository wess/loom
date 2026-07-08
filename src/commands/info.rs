//! `loom info` — show the full manifest for a skill.

use crate::commands::open_repo;
use crate::config::Config;
use crate::install::State;
use crate::output;
use anyhow::Result;
use colored::Colorize;

pub fn run(skill: &str) -> Result<()> {
    let config = Config::load()?;
    let repo = open_repo(&config);
    let entry = repo.get(skill)?;
    let m = &entry.manifest;

    println!("{} {}", m.name.bold().green(), m.version.dimmed());
    println!("{}", m.description);
    println!();
    output::detail("homepage", &m.homepage);
    if let Some(license) = &m.license {
        output::detail("license", license);
    }
    if !m.authors.is_empty() {
        output::detail("authors", &m.authors.join(", "));
    }
    if !m.keywords.is_empty() {
        output::detail("keywords", &m.keywords.join(", "));
    }
    if !m.compatibility.is_empty() {
        output::detail("agents", &m.compatibility.join(", "));
    }
    output::detail("source", &format!("{:?} {}", m.source.kind, m.source.url));
    if let Some(sub) = &m.source.subdir {
        output::detail("subdir", sub);
    }
    output::detail("manifest", &entry.path.display().to_string());

    let state = State::load()?;
    let installed: Vec<_> = state
        .installed
        .iter()
        .filter(|r| r.name == m.name)
        .map(|r| format!("{} ({})", r.agent, r.version))
        .collect();
    if !installed.is_empty() {
        output::detail("installed", &installed.join(", "));
    }
    Ok(())
}
