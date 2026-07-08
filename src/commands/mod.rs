//! Command handlers. Each submodule owns one subcommand and delegates to the
//! domain modules for the real work.

pub mod agents;
pub mod doctor;
pub mod generate;
pub mod index;
pub mod info;
pub mod init;
pub mod install;
pub mod lint;
pub mod list;
pub mod new;
pub mod publish;
pub mod remove;
pub mod search;
pub mod test;
pub mod update;
pub mod upgrade;

use crate::config::Config;
use crate::manifest::Manifest;
use crate::repo::Repo;
use anyhow::Result;
use std::path::Path;

/// Open the manifest repository described by config.
pub fn open_repo(config: &Config) -> Repo {
    Repo::open(config.manifest_dir())
}

/// Resolve a skill argument that may be a repo name or a local manifest path.
pub fn resolve_manifest(config: &Config, skill: &str) -> Result<Manifest> {
    let path = Path::new(skill);
    if path.exists() && path.is_file() {
        return Manifest::load(path);
    }
    let repo = open_repo(config);
    Ok(repo.get(skill)?.manifest)
}
