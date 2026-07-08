//! `loom publish` — open a pull request that adds a manifest to the skill
//! repository, the way `brew` helps you contribute a formula.
//!
//! Publishing mutates remote state (a fork, a branch, a PR), so it defaults to a
//! dry run that only prints the plan. Pass `--execute` to actually run it.

use crate::commands::resolve_manifest;
use crate::config::Config;
use crate::manifest::Manifest;
use crate::output;
use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;

pub fn run(skill: &str, repo: Option<&str>, execute: bool) -> Result<()> {
    let config = Config::load()?;
    let manifest = resolve_manifest(&config, skill)?;

    // A skill that fails validation must never reach a PR.
    manifest.validate()?;
    let source_file = locate_source(&config, skill, &manifest)?;

    let repo_url = repo
        .map(String::from)
        .or_else(|| config.repo_url.clone())
        .context("no repository configured; pass --repo <url> or set repo_url")?;
    let (owner, project) = parse_repo(&repo_url)?;

    let branch = format!("loom/{}", manifest.name);
    let dest = format!("skills/{}.yml", manifest.name);
    let title = format!("Add {} {}", manifest.name, manifest.version);
    let body = pr_body(&manifest);

    output::step(&format!(
        "Publishing {} {} to {owner}/{project}",
        manifest.name, manifest.version
    ));

    if !execute {
        print_plan(&owner, &project, &branch, &dest, &source_file, &title);
        println!();
        output::warn("dry run — nothing was changed. re-run with --execute to open the PR.");
        return Ok(());
    }

    ensure_gh()?;
    let login = gh_login()?;

    // 1. Fork (idempotent) and get a local clone of the fork.
    let workdir = std::env::temp_dir().join(format!("loom-publish-{}", manifest.name));
    if workdir.exists() {
        std::fs::remove_dir_all(&workdir).ok();
    }
    run_step(
        "fork the repository",
        "gh",
        &["repo", "fork", &format!("{owner}/{project}"), "--clone=false"],
    )?;
    run_step(
        "clone your fork",
        "gh",
        &["repo", "clone", &format!("{login}/{project}"), &workdir.to_string_lossy()],
    )?;

    // 2. Branch, drop the manifest in place, commit, push.
    let wd = workdir.to_string_lossy().to_string();
    run_step("create a branch", "git", &["-C", &wd, "checkout", "-b", &branch])?;

    let target = workdir.join("skills").join(format!("{}.yml", manifest.name));
    crate::paths::ensure_dir(target.parent().unwrap())?;
    std::fs::copy(&source_file, &target)
        .with_context(|| format!("cannot copy manifest into {}", target.display()))?;

    run_step("stage the manifest", "git", &["-C", &wd, "add", &dest])?;
    run_step("commit", "git", &["-C", &wd, "commit", "-m", &title])?;
    run_step("push to your fork", "git", &["-C", &wd, "push", "-u", "origin", &branch])?;

    // 3. Open the PR against the upstream repo.
    run_step(
        "open the pull request",
        "gh",
        &[
            "pr",
            "create",
            "--repo",
            &format!("{owner}/{project}"),
            "--head",
            &format!("{login}:{branch}"),
            "--title",
            &title,
            "--body",
            &body,
        ],
    )?;

    output::ok("Pull request opened");
    Ok(())
}

/// Find the on-disk manifest file to publish.
fn locate_source(config: &Config, skill: &str, manifest: &Manifest) -> Result<PathBuf> {
    let as_path = PathBuf::from(skill);
    if as_path.is_file() {
        return Ok(as_path);
    }
    let repo_file = config.manifest_dir().join(format!("{}.yml", manifest.name));
    if repo_file.exists() {
        return Ok(repo_file);
    }
    bail!(
        "cannot find a manifest file for '{}'. author it first with `loom new` or `loom init`.",
        manifest.name
    )
}

fn print_plan(owner: &str, project: &str, branch: &str, dest: &str, source: &PathBuf, title: &str) {
    let arrow = "→".dimmed();
    println!("  {arrow} fork {owner}/{project} to your account (if needed)");
    println!("  {arrow} clone your fork and branch {}", branch.cyan());
    println!("  {arrow} copy {} to {}", source.display(), dest.cyan());
    println!("  {arrow} commit \"{title}\" and push");
    println!("  {arrow} open a PR into {}/{}", owner, project);
}

fn pr_body(m: &Manifest) -> String {
    format!(
        "Adds the `{}` skill (v{}).\n\n\
         - **Homepage:** {}\n\
         - **Source:** {}\n\n\
         Authored with `loom`.",
        m.name, m.version, m.homepage, m.source.url
    )
}

fn ensure_gh() -> Result<()> {
    Command::new("gh")
        .arg("--version")
        .output()
        .context("GitHub CLI (`gh`) is required for publishing but was not found on PATH")?;
    Ok(())
}

fn gh_login() -> Result<String> {
    let out = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
        .context("failed to query GitHub identity via gh")?;
    if !out.status.success() {
        bail!("not logged in to GitHub. run `gh auth login` first.");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn run_step(label: &str, program: &str, args: &[&str]) -> Result<()> {
    output::step(label);
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {program}"))?;
    if !status.success() {
        bail!("`{program}` failed while trying to {label}");
    }
    Ok(())
}

/// Extract `owner` and `repo` from a github-style URL.
fn parse_repo(url: &str) -> Result<(String, String)> {
    let base = url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("git@")
        .replace(':', "/");
    let mut parts = base.split('/').filter(|s| !s.is_empty());
    let _host = parts.next();
    let owner = parts.next().context("could not parse owner from repo url")?;
    let repo = parts.next().context("could not parse repo name from repo url")?;
    Ok((owner.to_string(), repo.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repo_url_shapes() {
        assert_eq!(parse_repo("https://github.com/wess/loom").unwrap(), ("wess".into(), "loom".into()));
        assert_eq!(parse_repo("https://github.com/wess/loom.git").unwrap(), ("wess".into(), "loom".into()));
        assert_eq!(parse_repo("git@github.com:wess/loom.git").unwrap(), ("wess".into(), "loom".into()));
    }
}
