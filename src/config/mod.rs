//! User configuration: which AI agents Loom installs into, and where the skill
//! manifest repository lives.

use crate::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A single AI agent Loom can install skills into.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Human label, e.g. "Claude Code".
    pub label: String,
    /// Directory the agent loads skills from. May contain a leading `~`.
    pub skills_dir: String,
}

/// Persisted Loom configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Agent id -> agent definition.
    pub agents: BTreeMap<String, Agent>,
    /// The agent used when `--agent` is not given.
    pub default_agent: String,
    /// Local path to the manifest repository's `skills` folder.
    /// Defaults to the `skills` folder next to the running repo.
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Remote manifest repository to sync from (git url).
    #[serde(default)]
    pub repo_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut agents = BTreeMap::new();
        agents.insert(
            "claude-code".to_string(),
            Agent {
                label: "Claude Code".to_string(),
                skills_dir: "~/.claude/skills".to_string(),
            },
        );
        agents.insert(
            "codex".to_string(),
            Agent {
                label: "OpenAI Codex".to_string(),
                skills_dir: "~/.codex/skills".to_string(),
            },
        );
        agents.insert(
            "cursor".to_string(),
            Agent {
                label: "Cursor".to_string(),
                skills_dir: "~/.cursor/skills".to_string(),
            },
        );
        Config {
            agents,
            default_agent: "claude-code".to_string(),
            repo_path: None,
            repo_url: Some("https://github.com/loomskills/loom".to_string()),
        }
    }
}

impl Config {
    /// Load config from disk, creating a default if none exists yet.
    pub fn load() -> Result<Config> {
        let path = paths::config_file()?;
        if !path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let config: Config =
            serde_json::from_str(&text).context("config.json is malformed")?;
        Ok(config)
    }

    /// Persist config to disk.
    pub fn save(&self) -> Result<()> {
        let path = paths::config_file()?;
        paths::ensure_dir(path.parent().unwrap())?;
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, text)
            .with_context(|| format!("cannot write {}", path.display()))?;
        Ok(())
    }

    /// Resolve an agent by id, falling back to the default.
    pub fn resolve_agent(&self, id: Option<&str>) -> Result<(&str, &Agent)> {
        let key = id.unwrap_or(&self.default_agent);
        let agent = self.agents.get(key).with_context(|| {
            format!(
                "unknown agent '{key}'. known agents: {}",
                self.agents.keys().cloned().collect::<Vec<_>>().join(", ")
            )
        })?;
        // Return the canonical key from the map rather than the input slice.
        let canonical = self.agents.get_key_value(key).unwrap().0.as_str();
        Ok((canonical, agent))
    }

    /// Absolute skills directory for an agent.
    pub fn agent_skills_dir(&self, agent: &Agent) -> PathBuf {
        paths::expand_tilde(&agent.skills_dir)
    }

    /// Where local manifests live: explicit config, else `./skills` beside cwd.
    pub fn manifest_dir(&self) -> PathBuf {
        match &self.repo_path {
            Some(p) => paths::expand_tilde(p),
            None => std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("skills"),
        }
    }
}
