//! Installing, removing, and tracking skills on disk.
//!
//! Loom keeps a small JSON registry of what is installed where, so it can list,
//! upgrade, and cleanly remove skills across multiple agents.

use crate::config::{Agent, Config};
use crate::fetch;
use crate::manifest::Manifest;
use crate::paths;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A record of one installed skill for one agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub name: String,
    pub version: String,
    pub agent: String,
    pub path: String,
    pub source_url: String,
}

/// The persisted install registry.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub installed: Vec<Record>,
}

impl State {
    /// Load the registry, defaulting to empty.
    pub fn load() -> Result<State> {
        let path = paths::state_file()?;
        if !path.exists() {
            return Ok(State::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        Ok(serde_json::from_str(&text).context("state.json is malformed")?)
    }

    /// Persist the registry.
    pub fn save(&self) -> Result<()> {
        let path = paths::state_file()?;
        paths::ensure_dir(path.parent().unwrap())?;
        std::fs::write(&path, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("cannot write {}", path.display()))?;
        Ok(())
    }

    fn upsert(&mut self, record: Record) {
        if let Some(existing) = self
            .installed
            .iter_mut()
            .find(|r| r.name == record.name && r.agent == record.agent)
        {
            *existing = record;
        } else {
            self.installed.push(record);
        }
        self.installed
            .sort_by(|a, b| a.name.cmp(&b.name).then(a.agent.cmp(&b.agent)));
    }

    fn remove(&mut self, name: &str, agent: &str) -> Option<Record> {
        let idx = self
            .installed
            .iter()
            .position(|r| r.name == name && r.agent == agent)?;
        Some(self.installed.remove(idx))
    }
}

/// Install a skill described by `manifest` for `agent`.
/// Returns the directory the skill was written to.
pub fn install(
    config: &Config,
    manifest: &Manifest,
    agent_id: &str,
    agent: &Agent,
) -> Result<PathBuf> {
    manifest.validate()?;

    let payload = fetch::fetch(manifest)?;
    let target = config.agent_skills_dir(agent).join(&manifest.name);

    let result = (|| -> Result<PathBuf> {
        if target.exists() {
            std::fs::remove_dir_all(&target)
                .with_context(|| format!("cannot replace {}", target.display()))?;
        }
        paths::ensure_dir(&target)?;

        if manifest.install.files.is_empty() {
            copy_tree(&payload.root, &target)?;
        } else {
            for rel in &manifest.install.files {
                let from = payload.root.join(rel);
                let to = target.join(rel);
                if let Some(parent) = to.parent() {
                    paths::ensure_dir(parent)?;
                }
                std::fs::copy(&from, &to)
                    .with_context(|| format!("cannot copy {}", from.display()))?;
            }
        }

        let entry = target.join(&manifest.install.entry);
        if !entry.exists() {
            bail!(
                "entry file '{}' missing after install",
                manifest.install.entry
            );
        }
        Ok(target)
    })();

    payload.cleanup();
    let target = result?;

    let mut state = State::load()?;
    state.upsert(Record {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        agent: agent_id.to_string(),
        path: target.to_string_lossy().to_string(),
        source_url: manifest.source.url.clone(),
    });
    state.save()?;

    Ok(target)
}

/// Remove an installed skill for an agent.
pub fn uninstall(name: &str, agent_id: &str) -> Result<PathBuf> {
    let mut state = State::load()?;
    let record = state
        .remove(name, agent_id)
        .with_context(|| format!("'{name}' is not installed for agent '{agent_id}'"))?;
    let path = PathBuf::from(&record.path);
    if path.exists() {
        std::fs::remove_dir_all(&path)
            .with_context(|| format!("cannot remove {}", path.display()))?;
    }
    state.save()?;
    Ok(path)
}

/// Recursively copy a directory tree, skipping VCS metadata.
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    for entry in WalkDir::new(from).into_iter().filter_map(|e| e.ok()) {
        let rel = entry.path().strip_prefix(from).unwrap();
        if rel.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }
        let dest = to.join(rel);
        if entry.file_type().is_dir() {
            paths::ensure_dir(&dest)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = dest.parent() {
                paths::ensure_dir(parent)?;
            }
            std::fs::copy(entry.path(), &dest)
                .with_context(|| format!("cannot copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}
