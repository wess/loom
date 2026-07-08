//! `loom list` — show installed skills.

use crate::install::State;
use crate::output;
use anyhow::Result;
use colored::Colorize;

pub fn run(agent: Option<&str>) -> Result<()> {
    let state = State::load()?;
    let records: Vec<_> = state
        .installed
        .iter()
        .filter(|r| agent.is_none_or(|a| r.agent == a))
        .collect();

    if records.is_empty() {
        output::warn("no skills installed");
        return Ok(());
    }

    for record in records {
        println!(
            "{}  {}  {}",
            record.name.bold(),
            record.version.dimmed(),
            format!("[{}]", record.agent).blue()
        );
    }
    Ok(())
}
