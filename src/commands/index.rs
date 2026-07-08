//! `loom index` — build the JSON search index the website consumes.

use crate::commands::open_repo;
use crate::config::Config;
use crate::{output, site};
use anyhow::Result;
use std::path::PathBuf;

pub fn run(out: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    let repo = open_repo(&config);
    let index = site::build(&repo)?;

    let out = out
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("docs/skills.json"));
    site::write(&index, &out)?;

    output::ok(&format!("Indexed {} skills -> {}", index.count, out.display()));
    Ok(())
}
