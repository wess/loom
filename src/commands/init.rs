//! `loom init` — an interactive wizard that authors a manifest, optionally
//! importing field defaults by inspecting an existing skills repository.

use crate::config::Config;
use crate::manifest::{Install, Manifest, Severity, Source, SourceKind};
use crate::{generate, output, paths};
use anyhow::{Result, bail};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use std::io::IsTerminal;
use std::path::PathBuf;

const AGENTS: [&str; 3] = ["claude-code", "codex", "cursor"];

pub fn run(url: Option<&str>, ref_: Option<&str>, out: Option<&str>) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        bail!(
            "loom init is interactive and needs a terminal.\n\
             In scripts, use `loom new <name>` or `loom generate <url>` instead."
        );
    }

    let theme = ColorfulTheme::default();
    let config = Config::load()?;

    // Optionally import defaults from a real repo.
    let seed = seed_from_repo(&theme, url, ref_)?;

    output::step("Answer a few questions to author your manifest");

    let name: String = Input::with_theme(&theme)
        .with_prompt("name")
        .with_initial_text(seed.as_ref().map(|m| m.name.clone()).unwrap_or_default())
        .validate_with(|v: &String| validate_name(v))
        .interact_text()?;

    let description: String = Input::with_theme(&theme)
        .with_prompt("description")
        .with_initial_text(seed.as_ref().map(|m| m.description.clone()).unwrap_or_default())
        .interact_text()?;

    let homepage_default = seed
        .as_ref()
        .map(|m| m.homepage.clone())
        .unwrap_or_else(|| "https://github.com/you/your-skills".into());
    let homepage: String = Input::with_theme(&theme)
        .with_prompt("homepage")
        .with_initial_text(homepage_default)
        .interact_text()?;

    let author: String = Input::with_theme(&theme)
        .with_prompt("author (Name <email>), blank to skip")
        .with_initial_text(seed.as_ref().and_then(|m| m.authors.first().cloned()).unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    let license: String = Input::with_theme(&theme)
        .with_prompt("license (SPDX id), blank to skip")
        .with_initial_text(seed.as_ref().and_then(|m| m.license.clone()).unwrap_or_else(|| "MIT".into()))
        .allow_empty(true)
        .interact_text()?;

    let keywords: String = Input::with_theme(&theme)
        .with_prompt("keywords (comma separated)")
        .with_initial_text(seed.as_ref().map(|m| m.keywords.join(", ")).unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    // Which agents this skill supports.
    let compatibility = pick_agents(&theme, seed.as_ref())?;

    // Source.
    let kind_idx = Select::with_theme(&theme)
        .with_prompt("source type")
        .items(&["git", "archive"])
        .default(seed.as_ref().map(source_index).unwrap_or(0))
        .interact()?;
    let kind = if kind_idx == 0 { SourceKind::Git } else { SourceKind::Archive };

    let url_default = seed.as_ref().map(|m| m.source.url.clone()).unwrap_or_default();
    let source_url: String = Input::with_theme(&theme)
        .with_prompt(if kind == SourceKind::Git { "git url" } else { "archive url" })
        .with_initial_text(url_default)
        .interact_text()?;

    let (ref_field, sha256) = match kind {
        SourceKind::Git => {
            let r: String = Input::with_theme(&theme)
                .with_prompt("git ref (tag/branch/commit)")
                .with_initial_text(
                    seed.as_ref().and_then(|m| m.source.ref_.clone()).unwrap_or_else(|| "v0.1.0".into()),
                )
                .interact_text()?;
            (Some(r), None)
        }
        SourceKind::Archive => {
            let s: String = Input::with_theme(&theme)
                .with_prompt("archive sha256")
                .allow_empty(true)
                .interact_text()?;
            (None, if s.is_empty() { None } else { Some(s) })
        }
    };

    let subdir: String = Input::with_theme(&theme)
        .with_prompt("subdir within the source (blank if the root is the skill)")
        .with_initial_text(seed.as_ref().and_then(|m| m.source.subdir.clone()).unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    let entry: String = Input::with_theme(&theme)
        .with_prompt("entry file")
        .with_initial_text("SKILL.md")
        .interact_text()?;

    let manifest = Manifest {
        name: name.clone(),
        version: seed.as_ref().map(|m| m.version.clone()).unwrap_or_else(|| "0.1.0".into()),
        description,
        homepage,
        license: none_if_empty(license),
        authors: if author.is_empty() { vec![] } else { vec![author] },
        keywords: split_list(&keywords),
        compatibility,
        source: Source {
            kind,
            url: source_url,
            ref_: ref_field,
            sha256,
            subdir: none_if_empty(subdir),
        },
        install: Install { entry, files: vec![] },
    };

    // Surface any quality nits before writing.
    for problem in manifest.lint() {
        match problem.severity {
            Severity::Error => output::error(&problem.message),
            Severity::Warning => output::warn(&problem.message),
        }
    }

    let dir = match out {
        Some(o) => paths::expand_tilde(o),
        None => config.manifest_dir(),
    };
    let path: PathBuf = dir.join(format!("{name}.yml"));

    if path.exists()
        && !Confirm::with_theme(&theme)
            .with_prompt(format!("{} exists — overwrite?", path.display()))
            .default(false)
            .interact()?
    {
        output::warn("aborted");
        return Ok(());
    }

    paths::ensure_dir(&dir)?;
    let header = format!("# {name} — Loom skill manifest\n# Docs: https://wess.io/loom/docs.html\n\n");
    std::fs::write(&path, format!("{header}{}", manifest.to_yaml()?))?;
    output::ok(&format!("Wrote {}", path.display()));

    // Offer to verify the manifest actually resolves.
    if Confirm::with_theme(&theme)
        .with_prompt("test-fetch it now to confirm it resolves?")
        .default(true)
        .interact()?
    {
        crate::commands::test::run(&path.to_string_lossy())?;
    }

    Ok(())
}

/// Clone and inspect a repo to prefill answers, if the user wants to.
fn seed_from_repo(
    theme: &ColorfulTheme,
    url: Option<&str>,
    ref_: Option<&str>,
) -> Result<Option<Manifest>> {
    let url = match url {
        Some(u) => u.to_string(),
        None => {
            if !Confirm::with_theme(theme)
                .with_prompt("import defaults from an existing skills repo?")
                .default(false)
                .interact()?
            {
                return Ok(None);
            }
            Input::with_theme(theme)
                .with_prompt("repo url")
                .interact_text()?
        }
    };

    output::step(&format!("Inspecting {url}"));
    let mut found = generate::from_repo(&url, ref_)?;
    if found.is_empty() {
        return Ok(None);
    }
    if found.len() == 1 {
        return Ok(Some(found.remove(0)));
    }

    let names: Vec<String> = found.iter().map(|m| m.name.clone()).collect();
    let idx = Select::with_theme(theme)
        .with_prompt(format!("found {} skills — pick one to start from", found.len()))
        .items(&names)
        .default(0)
        .interact()?;
    Ok(Some(found.remove(idx)))
}

fn pick_agents(theme: &ColorfulTheme, seed: Option<&Manifest>) -> Result<Vec<String>> {
    let preselected: Vec<bool> = AGENTS
        .iter()
        .map(|a| match seed {
            Some(m) => m.compatibility.iter().any(|c| c == a),
            None => *a == "claude-code",
        })
        .collect();
    let chosen = MultiSelect::with_theme(theme)
        .with_prompt("compatible agents (space to toggle, enter to confirm)")
        .items(&AGENTS)
        .defaults(&preselected)
        .interact()?;
    Ok(chosen.into_iter().map(|i| AGENTS[i].to_string()).collect())
}

fn source_index(m: &Manifest) -> usize {
    match m.source.kind {
        SourceKind::Git => 0,
        SourceKind::Archive => 1,
    }
}

fn validate_name(v: &str) -> Result<(), String> {
    if v.trim().is_empty() {
        return Err("name cannot be empty".into());
    }
    if v != v.to_lowercase() {
        return Err("name must be lowercase".into());
    }
    if v.contains(char::is_whitespace) {
        return Err("name cannot contain whitespace".into());
    }
    Ok(())
}

fn none_if_empty(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

fn split_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
