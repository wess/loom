//! `loom update` — sync the local skill repository from its git remote.

use crate::config::Config;
use crate::{output, paths};
use anyhow::{Result, bail};
use std::process::Command;

pub fn run() -> Result<()> {
    let config = Config::load()?;
    let Some(url) = config.repo_url.clone() else {
        output::warn("no repo_url configured; nothing to sync");
        return Ok(());
    };

    let cache = paths::cache_dir()?.join("repo");
    paths::ensure_dir(cache.parent().unwrap())?;

    if cache.join(".git").exists() {
        output::step(&format!("Updating repository from {url}"));
        run_git(&["-C", &cache.to_string_lossy(), "pull", "--quiet", "--ff-only"])?;
    } else {
        output::step(&format!("Cloning repository from {url}"));
        std::fs::remove_dir_all(&cache).ok();
        run_git(&["clone", "--quiet", &url, &cache.to_string_lossy()])?;
    }

    // Point the config at the synced repo's skills folder if not overridden.
    let mut config = config;
    if config.repo_path.is_none() {
        config.repo_path = Some(cache.join("skills").to_string_lossy().to_string());
        config.save()?;
    }

    output::ok("Repository up to date");
    Ok(())
}

fn run_git(args: &[&str]) -> Result<()> {
    let status = Command::new("git").args(args).status()?;
    if !status.success() {
        bail!("git {} failed", args.first().copied().unwrap_or(""));
    }
    Ok(())
}
