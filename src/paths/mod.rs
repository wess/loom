//! Filesystem locations Loom reads from and writes to.
//!
//! Everything Loom owns lives under a single prefix (`~/.loom` by default,
//! overridable with `LOOM_HOME`), mirroring how Homebrew keeps a self contained
//! prefix.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Root of Loom's own state: config, cache, and the installed registry.
pub fn home() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("LOOM_HOME") {
        return Ok(PathBuf::from(custom));
    }
    let base = dirs::home_dir().context("cannot resolve home directory")?;
    Ok(base.join(".loom"))
}

/// The config file describing agents and repo settings.
pub fn config_file() -> Result<PathBuf> {
    Ok(home()?.join("config.json"))
}

/// The registry of currently installed skills.
pub fn state_file() -> Result<PathBuf> {
    Ok(home()?.join("state.json"))
}

/// Scratch space for clones and downloads.
pub fn cache_dir() -> Result<PathBuf> {
    Ok(home()?.join("cache"))
}

/// Ensure a directory exists, creating parents as needed.
pub fn ensure_dir(path: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("cannot create {}", path.display()))?;
    Ok(())
}

/// Expand a leading `~` in a configured path against the home directory.
pub fn expand_tilde(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(raw)
}
