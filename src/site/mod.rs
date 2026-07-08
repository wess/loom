//! Building the search index the website consumes.
//!
//! The gh-pages site is fully static, so search runs client side over a JSON
//! index that Loom generates from the `skills/` folder.

use crate::manifest::Manifest;
use crate::repo::Repo;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

/// One record in the website search index.
#[derive(Serialize)]
pub struct IndexEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub homepage: String,
    pub license: Option<String>,
    pub authors: Vec<String>,
    pub keywords: Vec<String>,
    pub compatibility: Vec<String>,
    pub source: String,
}

impl IndexEntry {
    fn from_manifest(m: &Manifest) -> Self {
        IndexEntry {
            name: m.name.clone(),
            version: m.version.clone(),
            description: m.description.clone(),
            homepage: m.homepage.clone(),
            license: m.license.clone(),
            authors: m.authors.clone(),
            keywords: m.keywords.clone(),
            compatibility: m.compatibility.clone(),
            source: m.source.url.clone(),
        }
    }
}

/// The full index document written to disk.
#[derive(Serialize)]
pub struct Index {
    pub generated_by: String,
    pub count: usize,
    pub skills: Vec<IndexEntry>,
}

/// Build the search index from a repo.
pub fn build(repo: &Repo) -> Result<Index> {
    let (entries, _errors) = repo.entries()?;
    let skills: Vec<IndexEntry> = entries
        .iter()
        .map(|e| IndexEntry::from_manifest(&e.manifest))
        .collect();
    Ok(Index {
        generated_by: format!("loom {}", env!("CARGO_PKG_VERSION")),
        count: skills.len(),
        skills,
    })
}

/// Write the index as pretty JSON to a path.
pub fn write(index: &Index, out: &Path) -> Result<()> {
    if let Some(parent) = out.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    let text = serde_json::to_string_pretty(index)?;
    std::fs::write(out, text)
        .with_context(|| format!("cannot write {}", out.display()))?;
    Ok(())
}
