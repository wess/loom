//! `loom generate` — synthesize manifests from a skills repository URL.

use crate::config::Config;
use crate::{generate, output, paths};
use anyhow::Result;
use colored::Colorize;

pub fn run(url: &str, ref_: Option<&str>, out: Option<&str>) -> Result<()> {
    output::step(&format!("Inspecting {url}"));
    let manifests = generate::from_repo(url, ref_)?;
    output::ok(&format!("Discovered {} skill(s)", manifests.len()));

    match out {
        Some(dir) => {
            let _config = Config::load()?;
            let dir = paths::expand_tilde(dir);
            paths::ensure_dir(&dir)?;
            for manifest in &manifests {
                let path = dir.join(format!("{}.yml", manifest.name));
                std::fs::write(&path, manifest.to_yaml()?)?;
                output::detail("wrote", &path.display().to_string());
            }
            output::detail("next", "review each manifest, then `loom lint`");
        }
        None => {
            for manifest in &manifests {
                println!("\n{}", format!("# {}.yml", manifest.name).dimmed());
                println!("{}", manifest.to_yaml()?);
            }
        }
    }
    Ok(())
}
