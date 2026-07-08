//! `loom agents` — list the agents Loom can install into.

use crate::config::Config;
use anyhow::Result;
use colored::Colorize;

pub fn run() -> Result<()> {
    let config = Config::load()?;
    for (id, agent) in &config.agents {
        let marker = if *id == config.default_agent { "*" } else { " " };
        println!(
            "{} {}  {}\n    {}",
            marker.green().bold(),
            id.bold(),
            agent.label.dimmed(),
            config.agent_skills_dir(agent).display()
        );
    }
    println!("\n{} = default agent", "*".green().bold());
    Ok(())
}
