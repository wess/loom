//! `loom upgrade` — reinstall installed skills whose repo version has moved on.

use crate::commands::open_repo;
use crate::config::Config;
use crate::install::{self, State};
use crate::output;
use anyhow::Result;

pub fn run(skill: Option<&str>, agent: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    let repo = open_repo(&config);
    let state = State::load()?;

    let targets: Vec<_> = state
        .installed
        .iter()
        .filter(|r| skill.is_none_or(|s| r.name == s))
        .filter(|r| agent.is_none_or(|a| r.agent == a))
        .cloned()
        .collect();

    if targets.is_empty() {
        output::warn("nothing installed to upgrade");
        return Ok(());
    }

    let mut upgraded = 0;
    for record in targets {
        let entry = match repo.get(&record.name) {
            Ok(e) => e,
            Err(_) => {
                output::warn(&format!("{} no longer in repo, skipping", record.name));
                continue;
            }
        };
        if entry.manifest.version == record.version {
            output::detail(&record.name, &format!("up to date ({})", record.version));
            continue;
        }

        let (agent_id, agent_def) = config.resolve_agent(Some(&record.agent))?;
        output::step(&format!(
            "Upgrading {} {} -> {} [{}]",
            record.name, record.version, entry.manifest.version, record.agent
        ));
        install::install(&config, &entry.manifest, agent_id, agent_def)?;
        upgraded += 1;
    }

    output::ok(&format!("{upgraded} skill(s) upgraded"));
    Ok(())
}
