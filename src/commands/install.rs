//! `loom install` — fetch and place a skill for an agent.

use crate::commands::resolve_manifest;
use crate::config::Config;
use crate::{install, output};
use anyhow::Result;

pub fn run(skill: &str, agent: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    let manifest = resolve_manifest(&config, skill)?;
    let (agent_id, agent) = config.resolve_agent(agent)?;

    output::step(&format!(
        "Installing {} {} for {}",
        manifest.name, manifest.version, agent.label
    ));

    let target = install::install(&config, &manifest, agent_id, agent)?;
    output::ok(&format!("Installed to {}", target.display()));
    Ok(())
}
