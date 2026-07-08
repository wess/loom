//! Skill manifest: the `<skill_name>.yml` contract that lives in a Loom repo.
//!
//! A manifest never contains the skill payload itself. It only describes how to
//! fetch and install a skill from a remote source, in the spirit of a Homebrew
//! formula.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A fully described skill package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Unique skill identifier. Lowercase, dot/dash free where possible.
    pub name: String,
    /// Semver-ish version string.
    pub version: String,
    /// One line summary shown in search and info.
    pub description: String,
    /// Project homepage.
    pub homepage: String,
    /// SPDX license identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// One or more authors, free form `Name <email>` strings.
    #[serde(default)]
    pub authors: Vec<String>,
    /// Search keywords / tags.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// AI agents this skill is known to work with (claude-code, codex, ...).
    #[serde(default)]
    pub compatibility: Vec<String>,
    /// Where and how to obtain the skill payload.
    pub source: Source,
    /// How to lay the skill down once fetched.
    #[serde(default)]
    pub install: Install,
}

/// The remote origin of a skill payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Fetch strategy.
    #[serde(rename = "type")]
    pub kind: SourceKind,
    /// Clone/download URL.
    pub url: String,
    /// Git ref (tag, branch, or commit). Git sources only.
    #[serde(rename = "ref", default, skip_serializing_if = "Option::is_none")]
    pub ref_: Option<String>,
    /// Expected sha256 of the archive. Archive sources only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Sub directory inside the repo/archive that holds the skill.
    /// Empty means the repository root is the skill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdir: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// Clone a git repository.
    Git,
    /// Download and extract a `.tar.gz` archive.
    Archive,
}

/// Placement rules applied after a payload is fetched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Install {
    /// The entry file an agent loads first (e.g. `SKILL.md`).
    #[serde(default = "default_entry")]
    pub entry: String,
    /// Optional explicit relative paths to include. Empty copies everything.
    #[serde(default)]
    pub files: Vec<String>,
}

impl Default for Install {
    fn default() -> Self {
        Install { entry: default_entry(), files: Vec::new() }
    }
}

fn default_entry() -> String {
    "SKILL.md".to_string()
}

/// How serious a lint finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Blocks install.
    Error,
    /// Advisory; does not block install.
    Warning,
}

/// A single lint finding.
#[derive(Debug, Clone)]
pub struct Problem {
    pub severity: Severity,
    pub message: String,
}

impl Problem {
    fn error(message: &str) -> Problem {
        Problem { severity: Severity::Error, message: message.to_string() }
    }
    fn warn(message: &str) -> Problem {
        Problem { severity: Severity::Warning, message: message.to_string() }
    }
}

impl Manifest {
    /// Parse a manifest from a YAML string.
    pub fn from_yaml(text: &str) -> Result<Manifest> {
        let manifest: Manifest =
            serde_yaml_ng::from_str(text).context("manifest is not valid YAML")?;
        Ok(manifest)
    }

    /// Read and parse a manifest file from disk.
    pub fn load(path: &Path) -> Result<Manifest> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read manifest {}", path.display()))?;
        Manifest::from_yaml(&text)
            .with_context(|| format!("in {}", path.display()))
    }

    /// Serialize back to YAML.
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml_ng::to_string(self)?)
    }

    /// Validate the semantic rules a well formed manifest must satisfy.
    /// Returns every problem found; an empty list means a spotless manifest.
    /// Errors make a manifest uninstallable; warnings are advisory quality nits.
    pub fn lint(&self) -> Vec<Problem> {
        let mut p = Vec::new();

        if self.name.trim().is_empty() {
            p.push(Problem::error("name is empty"));
        }
        if self.name != self.name.to_lowercase() {
            p.push(Problem::error("name should be lowercase"));
        }
        if self.name.contains(char::is_whitespace) {
            p.push(Problem::error("name must not contain whitespace"));
        }
        if self.version.trim().is_empty() {
            p.push(Problem::error("version is empty"));
        }
        if self.description.trim().is_empty() {
            p.push(Problem::error("description is empty"));
        }
        if !self.homepage.starts_with("http") {
            p.push(Problem::error("homepage should be an http(s) url"));
        }
        if !self.source.url.starts_with("http") && !self.source.url.starts_with("git") {
            p.push(Problem::error("source.url should be a url"));
        }
        if self.install.entry.trim().is_empty() {
            p.push(Problem::error("install.entry is empty"));
        }
        if let SourceKind::Archive = self.source.kind {
            if self.source.sha256.is_none() {
                p.push(Problem::error("archive source must declare a sha256"));
            }
        }

        // Advisory quality checks.
        if self.description.len() > 500 {
            p.push(Problem::warn("description is long; consider tightening it"));
        }
        if self.authors.is_empty() {
            p.push(Problem::warn("no authors listed"));
        }
        if let SourceKind::Git = self.source.kind {
            if self.source.ref_.is_none() {
                p.push(Problem::warn("git source is unpinned; consider a tag or commit"));
            }
        }

        p
    }

    /// Validate strictly. Only hard errors fail; warnings are tolerated.
    pub fn validate(&self) -> Result<()> {
        let errors: Vec<_> = self
            .lint()
            .into_iter()
            .filter(|p| p.severity == Severity::Error)
            .map(|p| p.message)
            .collect();
        if !errors.is_empty() {
            bail!("invalid manifest:\n  - {}", errors.join("\n  - "));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
name: pdf
version: 1.0.0
description: Read and fill PDFs.
homepage: https://example.com
authors: [Jane <jane@example.com>]
source:
  type: git
  url: https://github.com/acme/skills
  ref: v1.0.0
  subdir: skills/pdf
install:
  entry: SKILL.md
"#;

    fn sample() -> Manifest {
        Manifest::from_yaml(SAMPLE).unwrap()
    }

    #[test]
    fn parses_ref_via_rename() {
        let m = sample();
        assert_eq!(m.source.ref_.as_deref(), Some("v1.0.0"));
        assert_eq!(m.source.subdir.as_deref(), Some("skills/pdf"));
        assert_eq!(m.install.entry, "SKILL.md");
    }

    #[test]
    fn clean_manifest_has_no_errors() {
        let errors = sample()
            .lint()
            .into_iter()
            .filter(|p| p.severity == Severity::Error)
            .count();
        assert_eq!(errors, 0);
        assert!(sample().validate().is_ok());
    }

    #[test]
    fn uppercase_name_is_an_error() {
        let mut m = sample();
        m.name = "PDF".into();
        assert!(m.validate().is_err());
    }

    #[test]
    fn archive_without_checksum_is_an_error() {
        let mut m = sample();
        m.source.kind = SourceKind::Archive;
        m.source.sha256 = None;
        assert!(m.validate().is_err());
    }

    #[test]
    fn missing_authors_is_only_a_warning() {
        let mut m = sample();
        m.authors.clear();
        assert!(m.validate().is_ok());
        assert!(
            m.lint().iter().any(|p| p.severity == Severity::Warning)
        );
    }

    #[test]
    fn yaml_roundtrips() {
        let yaml = sample().to_yaml().unwrap();
        let back = Manifest::from_yaml(&yaml).unwrap();
        assert_eq!(back.name, "pdf");
        assert_eq!(back.source.ref_.as_deref(), Some("v1.0.0"));
    }
}
