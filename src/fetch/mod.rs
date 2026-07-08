//! Fetching skill payloads from remote sources.
//!
//! Git sources shell out to the system `git` (like Homebrew), archive sources
//! stream over HTTP with `ureq`, verify a checksum, and unpack a gzip tarball.

use crate::manifest::{Manifest, SourceKind};
use crate::paths;
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A fetched payload rooted at the skill's subdir, ready to install.
pub struct Payload {
    /// The directory that holds the skill files (subdir already applied).
    pub root: PathBuf,
    /// Temp directory guard; kept alive so it is not cleaned mid install.
    _scratch: PathBuf,
}

impl Payload {
    /// Remove the scratch directory backing this payload.
    pub fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self._scratch);
    }
}

/// Fetch a manifest's source into a scratch directory and return its payload.
pub fn fetch(manifest: &Manifest) -> Result<Payload> {
    let scratch = scratch_dir(&manifest.name)?;
    if scratch.exists() {
        std::fs::remove_dir_all(&scratch).ok();
    }
    paths::ensure_dir(&scratch)?;

    let checkout = match manifest.source.kind {
        SourceKind::Git => fetch_git(manifest, &scratch)?,
        SourceKind::Archive => fetch_archive(manifest, &scratch)?,
    };

    let root = match &manifest.source.subdir {
        Some(sub) if !sub.is_empty() => checkout.join(sub),
        _ => checkout,
    };
    if !root.exists() {
        bail!("source subdir not found: {}", root.display());
    }
    Ok(Payload { root, _scratch: scratch })
}

fn scratch_dir(name: &str) -> Result<PathBuf> {
    Ok(paths::cache_dir()?.join("build").join(name))
}

fn fetch_git(manifest: &Manifest, scratch: &Path) -> Result<PathBuf> {
    let checkout = scratch.join("checkout");
    let mut clone = Command::new("git");
    clone
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--quiet");
    if let Some(reference) = &manifest.source.ref_ {
        clone.arg("--branch").arg(reference);
    }
    clone.arg(&manifest.source.url).arg(&checkout);

    let status = clone
        .status()
        .context("failed to run git (is it installed?)")?;
    if !status.success() {
        // A commit sha cannot be shallow cloned by branch; retry full clone.
        std::fs::remove_dir_all(&checkout).ok();
        let full = Command::new("git")
            .arg("clone")
            .arg("--quiet")
            .arg(&manifest.source.url)
            .arg(&checkout)
            .status()
            .context("failed to run git")?;
        if !full.success() {
            bail!("git clone failed for {}", manifest.source.url);
        }
        if let Some(reference) = &manifest.source.ref_ {
            let ok = Command::new("git")
                .arg("-C")
                .arg(&checkout)
                .arg("checkout")
                .arg("--quiet")
                .arg(reference)
                .status()
                .context("failed to run git checkout")?;
            if !ok.success() {
                bail!("git ref not found: {reference}");
            }
        }
    }
    Ok(checkout)
}

fn fetch_archive(manifest: &Manifest, scratch: &Path) -> Result<PathBuf> {
    let bytes = download(&manifest.source.url)?;

    if let Some(expected) = &manifest.source.sha256 {
        let actual = sha256_hex(&bytes);
        if !expected.eq_ignore_ascii_case(&actual) {
            bail!("sha256 mismatch\n  expected: {expected}\n  actual:   {actual}");
        }
    }

    let checkout = scratch.join("checkout");
    paths::ensure_dir(&checkout)?;
    let decoder = flate2::read::GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(&checkout)
        .context("failed to extract archive")?;

    // Many archives wrap contents in a single top level directory. Descend into
    // it so `subdir` resolves relative to the real project root.
    Ok(flatten_single_dir(checkout))
}

/// If a directory contains exactly one child directory and nothing else,
/// return that child; otherwise return the directory unchanged.
fn flatten_single_dir(dir: PathBuf) -> PathBuf {
    let entries: Vec<_> = match std::fs::read_dir(&dir) {
        Ok(read) => read.flatten().collect(),
        Err(_) => return dir,
    };
    if entries.len() == 1 && entries[0].path().is_dir() {
        return entries[0].path();
    }
    dir
}

/// Download a URL into memory.
pub fn download(url: &str) -> Result<Vec<u8>> {
    let mut response = ureq::get(url)
        .call()
        .with_context(|| format!("failed to GET {url}"))?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .read_to_end(&mut bytes)
        .context("failed to read response body")?;
    Ok(bytes)
}

/// Lowercase hex sha256 of a byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
