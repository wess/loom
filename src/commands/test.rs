//! `loom test` — fetch and stage a skill in a scratch dir to prove the manifest
//! actually resolves, without touching any agent's skills folder.

use crate::commands::resolve_manifest;
use crate::config::Config;
use crate::{fetch, output};
use anyhow::{Result, bail};
use walkdir::WalkDir;

pub fn run(skill: &str) -> Result<()> {
    let config = Config::load()?;
    let manifest = resolve_manifest(&config, skill)?;

    output::step(&format!("Testing {} {}", manifest.name, manifest.version));

    for problem in manifest.lint() {
        output::warn(&problem.message);
    }

    let payload = fetch::fetch(&manifest)?;
    let entry = payload.root.join(&manifest.install.entry);
    let entry_ok = entry.exists();

    let file_count = WalkDir::new(&payload.root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    output::detail("fetched", &format!("{file_count} files"));
    output::detail("entry", &manifest.install.entry);
    payload.cleanup();

    if !entry_ok {
        bail!("entry file '{}' not present in source", manifest.install.entry);
    }
    output::ok("manifest resolves and entry file is present");
    Ok(())
}
