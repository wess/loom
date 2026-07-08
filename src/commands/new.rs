//! `loom new` — scaffold a manifest to author from.

use crate::config::Config;
use crate::manifest::{Install, Manifest, Source, SourceKind};
use crate::{output, paths};
use anyhow::{Result, bail};
use std::path::PathBuf;

pub fn run(name: &str, out: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    let name = name.to_lowercase();

    let manifest = Manifest {
        name: name.clone(),
        version: "0.1.0".into(),
        description: "TODO: one line description of the skill".into(),
        homepage: "https://github.com/you/your-skills".into(),
        license: Some("MIT".into()),
        authors: vec!["Your Name <you@example.com>".into()],
        keywords: vec![],
        compatibility: vec!["claude-code".into()],
        source: Source {
            kind: SourceKind::Git,
            url: "https://github.com/you/your-skills".into(),
            ref_: Some("v0.1.0".into()),
            sha256: None,
            subdir: Some(name.clone()),
        },
        install: Install {
            entry: "SKILL.md".into(),
            files: vec![],
        },
    };

    let dir = match out {
        Some(o) => paths::expand_tilde(o),
        None => config.manifest_dir(),
    };
    paths::ensure_dir(&dir)?;
    let path: PathBuf = dir.join(format!("{name}.yml"));
    if path.exists() {
        bail!("{} already exists", path.display());
    }

    let header = format!(
        "# {name} — Loom skill manifest\n# Docs: https://loomskills.dev/docs/manifest\n\n"
    );
    std::fs::write(&path, format!("{header}{}", manifest.to_yaml()?))?;

    output::ok(&format!("Wrote {}", path.display()));
    output::detail("next", "edit source.url / subdir, then `loom lint` and `loom test`");
    Ok(())
}
