//! Generating manifests by inspecting a skills repository.
//!
//! Given a git URL, Loom clones the repo, discovers skill directories (any folder
//! containing a `SKILL.md`), reads their front matter, and emits a ready to edit
//! manifest per skill. This is the authoring shortcut, akin to `brew create`.

use crate::manifest::{Install, Manifest, Source, SourceKind};
use crate::paths;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Discover skills in a repo URL and return a generated manifest for each.
pub fn from_repo(url: &str, ref_: Option<&str>) -> Result<Vec<Manifest>> {
    let checkout = clone(url, ref_)?;
    let result = discover(&checkout, url, ref_);
    std::fs::remove_dir_all(&checkout).ok();
    result
}

fn clone(url: &str, ref_: Option<&str>) -> Result<PathBuf> {
    let dest = paths::cache_dir()?.join("generate").join(slug(url));
    if dest.exists() {
        std::fs::remove_dir_all(&dest).ok();
    }
    paths::ensure_dir(dest.parent().unwrap())?;

    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1").arg("--quiet");
    if let Some(reference) = ref_ {
        cmd.arg("--branch").arg(reference);
    }
    cmd.arg(url).arg(&dest);
    let status = cmd.status().context("failed to run git")?;
    if !status.success() {
        bail!("git clone failed for {url}");
    }
    Ok(dest)
}

fn discover(root: &Path, url: &str, ref_: Option<&str>) -> Result<Vec<Manifest>> {
    let mut skills = Vec::new();
    let license = detect_license(root);

    for entry in WalkDir::new(root)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() != "SKILL.md" || !entry.file_type().is_file() {
            continue;
        }
        let skill_dir = entry.path().parent().unwrap();
        let subdir = skill_dir
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");

        let name = skill_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "skill".into());
        // A root level SKILL.md yields a repo named skill.
        let name = if subdir.is_empty() {
            slug(url).trim_end_matches(".git").to_string()
        } else {
            name
        };

        let front = read_front_matter(entry.path());
        skills.push(Manifest {
            name: front.name.unwrap_or(name).to_lowercase(),
            version: version_from_ref(ref_),
            description: front
                .description
                .unwrap_or_else(|| "TODO: describe this skill".into()),
            homepage: normalize_homepage(url),
            license: license.clone(),
            authors: owner(url).into_iter().collect(),
            keywords: front.keywords,
            compatibility: vec!["claude-code".into()],
            source: Source {
                kind: SourceKind::Git,
                url: url.to_string(),
                ref_: ref_.map(String::from),
                sha256: None,
                subdir: if subdir.is_empty() { None } else { Some(subdir) },
            },
            install: Install {
                entry: "SKILL.md".into(),
                files: Vec::new(),
            },
        });
    }

    if skills.is_empty() {
        bail!("no SKILL.md files found under {}", root.display());
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// Minimal YAML front matter extracted from a SKILL.md.
#[derive(Default)]
struct FrontMatter {
    name: Option<String>,
    description: Option<String>,
    keywords: Vec<String>,
}

fn read_front_matter(path: &Path) -> FrontMatter {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return FrontMatter::default(),
    };
    let trimmed = text.trim_start();
    if !trimmed.starts_with("---") {
        return FrontMatter::default();
    }
    let body = &trimmed[3..];
    let end = match body.find("\n---") {
        Some(idx) => idx,
        None => return FrontMatter::default(),
    };
    let yaml = &body[..end];

    // Parse the block as real YAML so folded/literal scalars (`>-`, `|`),
    // quoted multi-line values, and list forms all resolve correctly.
    let value: serde_yaml_ng::Value = match serde_yaml_ng::from_str(yaml) {
        Ok(v) => v,
        Err(_) => return FrontMatter::default(),
    };

    let string_at = |key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let mut front = FrontMatter::default();
    front.name = string_at("name");
    front.description = string_at("description");
    for key in ["keywords", "tags"] {
        match value.get(key) {
            Some(serde_yaml_ng::Value::Sequence(items)) => {
                front.keywords = items
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            Some(serde_yaml_ng::Value::String(s)) => {
                front.keywords = s
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            _ => continue,
        }
        if !front.keywords.is_empty() {
            break;
        }
    }
    front
}

/// Guess a browsable homepage for common git hosts.
fn normalize_homepage(url: &str) -> String {
    let base = url.trim_end_matches(".git");
    if let Some(rest) = base.strip_prefix("git@github.com:") {
        return format!("https://github.com/{rest}");
    }
    base.to_string()
}

/// Sniff a common license file at the repo root and map it to an SPDX id.
fn detect_license(root: &Path) -> Option<String> {
    for name in ["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"] {
        let path = root.join(name);
        if let Ok(text) = std::fs::read_to_string(&path) {
            let head = text.to_lowercase();
            let id = if head.contains("apache license") {
                "Apache-2.0"
            } else if head.contains("mit license") || head.contains("permission is hereby granted") {
                "MIT"
            } else if head.contains("mozilla public license") {
                "MPL-2.0"
            } else if head.contains("gnu general public license") {
                "GPL-3.0"
            } else if head.contains("bsd") {
                "BSD-3-Clause"
            } else {
                continue;
            };
            return Some(id.to_string());
        }
    }
    None
}

/// Best effort owner/org extracted from a github-style URL, used to seed the
/// authors list. Returns None when it cannot be determined.
fn owner(url: &str) -> Option<String> {
    let base = url
        .trim_end_matches(".git")
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("git@")
        .replace(':', "/");
    let mut parts = base.split('/').filter(|s| !s.is_empty());
    let _host = parts.next()?;
    let owner = parts.next()?;
    Some(owner.to_string())
}

/// Derive a version string from a git ref. A semver-looking tag (`v1.2.0`,
/// `1.2.0`) becomes the version; a branch name (`main`) does not.
fn version_from_ref(reference: Option<&str>) -> String {
    match reference {
        Some(r) => {
            let stripped = r.trim_start_matches('v');
            let looks_versioned = stripped
                .split('.')
                .next()
                .map(|first| first.chars().all(|c| c.is_ascii_digit()) && !first.is_empty())
                .unwrap_or(false);
            if looks_versioned {
                stripped.to_string()
            } else {
                "0.1.0".to_string()
            }
        }
        None => "0.1.0".to_string(),
    }
}

/// A filesystem safe slug derived from a URL's last path segment.
fn slug(url: &str) -> String {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git")
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_tags_become_versions() {
        assert_eq!(version_from_ref(Some("v1.2.3")), "1.2.3");
        assert_eq!(version_from_ref(Some("1.2.3")), "1.2.3");
    }

    #[test]
    fn branch_names_fall_back_to_default() {
        assert_eq!(version_from_ref(Some("main")), "0.1.0");
        assert_eq!(version_from_ref(Some("develop")), "0.1.0");
        assert_eq!(version_from_ref(None), "0.1.0");
    }

    #[test]
    fn owner_parses_common_url_shapes() {
        assert_eq!(owner("https://github.com/anthropics/skills").as_deref(), Some("anthropics"));
        assert_eq!(owner("https://github.com/anthropics/skills.git").as_deref(), Some("anthropics"));
        assert_eq!(owner("git@github.com:anthropics/skills.git").as_deref(), Some("anthropics"));
    }

    #[test]
    fn homepage_normalizes_ssh_urls() {
        assert_eq!(
            normalize_homepage("git@github.com:acme/skills.git"),
            "https://github.com/acme/skills"
        );
    }

    fn front_matter(md: &str) -> FrontMatter {
        let dir = std::env::temp_dir().join(format!("loom-fm-{}", md.len()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("SKILL.md");
        std::fs::write(&path, md).unwrap();
        let front = read_front_matter(&path);
        std::fs::remove_dir_all(&dir).ok();
        front
    }

    #[test]
    fn folded_scalar_description_is_read() {
        let front = front_matter(
            "---\nname: nextjs\ndescription: >-\n  Build Next.js apps with\n  server components.\nkeywords:\n  - nextjs\n  - react\n---\n\nbody\n",
        );
        assert_eq!(front.name.as_deref(), Some("nextjs"));
        assert_eq!(
            front.description.as_deref(),
            Some("Build Next.js apps with server components.")
        );
        assert_eq!(front.keywords, vec!["nextjs", "react"]);
    }

    #[test]
    fn plain_and_quoted_front_matter_still_parses() {
        let front = front_matter(
            "---\nname: pdf\ndescription: \"Read and fill PDFs.\"\ntags: [docs, pdf]\n---\n",
        );
        assert_eq!(front.name.as_deref(), Some("pdf"));
        assert_eq!(front.description.as_deref(), Some("Read and fill PDFs."));
        assert_eq!(front.keywords, vec!["docs", "pdf"]);
    }
}
