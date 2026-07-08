//! The manifest repository: the `skills/` folder of `<name>.yml` files that Loom
//! searches and installs from. Analogous to a Homebrew tap.

use crate::manifest::Manifest;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// A manifest paired with the file it was read from.
pub struct Entry {
    pub manifest: Manifest,
    pub path: PathBuf,
}

/// A view over a directory of manifest files.
pub struct Repo {
    dir: PathBuf,
}

impl Repo {
    /// Open a repository rooted at a `skills` directory.
    pub fn open(dir: PathBuf) -> Self {
        Repo { dir }
    }

    /// The directory backing this repo.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Load every valid manifest in the directory, sorted by name.
    /// Malformed files are collected as errors rather than aborting the walk.
    pub fn entries(&self) -> Result<(Vec<Entry>, Vec<(PathBuf, String)>)> {
        if !self.dir.exists() {
            bail!("manifest directory not found: {}", self.dir.display());
        }
        let mut entries = Vec::new();
        let mut errors = Vec::new();
        for file in manifest_files(&self.dir)? {
            match Manifest::load(&file) {
                Ok(manifest) => entries.push(Entry { manifest, path: file }),
                Err(err) => errors.push((file, format!("{err:#}"))),
            }
        }
        entries.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        Ok((entries, errors))
    }

    /// Find a single manifest by exact skill name.
    pub fn get(&self, name: &str) -> Result<Entry> {
        let direct = self.dir.join(format!("{name}.yml"));
        let alt = self.dir.join(format!("{name}.yaml"));
        let path = if direct.exists() {
            direct
        } else if alt.exists() {
            alt
        } else {
            bail!("skill '{name}' not found in {}", self.dir.display());
        };
        let manifest = Manifest::load(&path)?;
        Ok(Entry { manifest, path })
    }

    /// Rank manifests against a free text query. Higher score first.
    pub fn search(&self, query: &str) -> Result<Vec<(Entry, i32)>> {
        let needle = query.to_lowercase();
        let (entries, _) = self.entries()?;
        let mut hits: Vec<(Entry, i32)> = entries
            .into_iter()
            .filter_map(|entry| {
                let score = score(&entry.manifest, &needle);
                if score > 0 { Some((entry, score)) } else { None }
            })
            .collect();
        hits.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.manifest.name.cmp(&b.0.manifest.name)));
        Ok(hits)
    }
}

/// Score a manifest against an already lowercased query.
fn score(manifest: &Manifest, needle: &str) -> i32 {
    let mut score = 0;
    let name = manifest.name.to_lowercase();
    if name == needle {
        score += 100;
    } else if name.starts_with(needle) {
        score += 50;
    } else if name.contains(needle) {
        score += 30;
    }
    if manifest.description.to_lowercase().contains(needle) {
        score += 10;
    }
    for keyword in &manifest.keywords {
        if keyword.to_lowercase().contains(needle) {
            score += 15;
        }
    }
    score
}

/// Every `.yml`/`.yaml` file directly inside a directory.
fn manifest_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
    {
        let path = entry?.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "yml" || ext == "yaml" {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}
