//! `loom search` — rank repository skills against a query.

use crate::commands::open_repo;
use crate::config::Config;
use crate::output;
use anyhow::Result;
use colored::Colorize;

pub fn run(query: &str) -> Result<()> {
    let config = Config::load()?;
    let repo = open_repo(&config);
    let hits = repo.search(query)?;

    if hits.is_empty() {
        output::warn(&format!("no skills matching '{query}'"));
        return Ok(());
    }

    for (entry, _score) in hits {
        let m = &entry.manifest;
        println!(
            "{} {}\n    {}",
            m.name.bold().green(),
            m.version.dimmed(),
            m.description
        );
    }
    Ok(())
}
