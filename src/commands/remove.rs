//! `loom remove` — uninstall a skill for an agent.

use crate::config::Config;
use crate::{install, output};
use anyhow::Result;

pub fn run(skill: &str, agent: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    let (agent_id, agent) = config.resolve_agent(agent)?;

    output::step(&format!("Removing {} from {}", skill, agent.label));
    let path = install::uninstall(skill, agent_id)?;
    output::ok(&format!("Removed {}", path.display()));
    Ok(())
}
