//! Command line surface: argument parsing and dispatch.

use crate::commands;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

/// Loom — a package manager for AI skills.
#[derive(Parser)]
#[command(
    name = "loom",
    version,
    about = "A package manager for AI skills (Claude Code, Codex, and friends).",
    long_about = "Loom installs, inspects, and authors reusable skills for AI agents.\n\
                  Manifests describe how to fetch a skill; Loom weaves them into place."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Install a skill for an agent.
    Install {
        /// Skill name from the repo, or a path/URL to a manifest.
        skill: String,
        /// Target agent (defaults to the configured default).
        #[arg(short, long)]
        agent: Option<String>,
    },
    /// Remove an installed skill.
    #[command(alias = "uninstall", alias = "rm")]
    Remove {
        skill: String,
        #[arg(short, long)]
        agent: Option<String>,
    },
    /// List installed skills.
    #[command(alias = "ls")]
    List {
        /// Only show skills for this agent.
        #[arg(short, long)]
        agent: Option<String>,
    },
    /// Search the skill repository.
    Search { query: String },
    /// Show details for a skill.
    Info { skill: String },
    /// Sync the local skill repository from its remote.
    Update,
    /// Upgrade installed skills to the repo's version.
    Upgrade {
        /// A single skill; omit to upgrade all.
        skill: Option<String>,
        #[arg(short, long)]
        agent: Option<String>,
    },
    /// Scaffold a new manifest in the repository.
    New {
        name: String,
        /// Where to write it (defaults to the repo's skills folder).
        #[arg(short, long)]
        out: Option<String>,
    },
    /// Validate one manifest file, or every manifest in the repo.
    Lint {
        /// A manifest file; omit to lint the whole repo.
        path: Option<String>,
    },
    /// Fetch and stage a skill without installing it, to verify the manifest.
    Test {
        /// Skill name from the repo, or a path to a manifest file.
        skill: String,
    },
    /// Generate manifests by inspecting a skills repository URL.
    #[command(alias = "create")]
    Generate {
        /// Git URL of the skills repository (positional form).
        #[arg(value_name = "URL")]
        url: Option<String>,
        /// Git URL of the skills repository (flag form, alternative to positional).
        #[arg(long = "url", value_name = "URL", conflicts_with = "url")]
        url_flag: Option<String>,
        /// Ref (tag/branch) to pin.
        #[arg(short, long)]
        r#ref: Option<String>,
        /// Directory to write generated manifests into (defaults to stdout).
        #[arg(short, long)]
        out: Option<String>,
    },
    /// Build the website search index from the repo.
    Index {
        /// Output path (defaults to docs/skills.json).
        #[arg(short, long)]
        out: Option<String>,
    },
    /// List the agents Loom can install into.
    Agents,
    /// Check the environment for common problems.
    Doctor,
}

/// Parse arguments and run the requested command.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Install { skill, agent } => commands::install::run(&skill, agent.as_deref()),
        Command::Remove { skill, agent } => commands::remove::run(&skill, agent.as_deref()),
        Command::List { agent } => commands::list::run(agent.as_deref()),
        Command::Search { query } => commands::search::run(&query),
        Command::Info { skill } => commands::info::run(&skill),
        Command::Update => commands::update::run(),
        Command::Upgrade { skill, agent } => {
            commands::upgrade::run(skill.as_deref(), agent.as_deref())
        }
        Command::New { name, out } => commands::new::run(&name, out.as_deref()),
        Command::Lint { path } => commands::lint::run(path.as_deref()),
        Command::Test { skill } => commands::test::run(&skill),
        Command::Generate { url, url_flag, r#ref, out } => {
            let url = url
                .or(url_flag)
                .context("provide a repo URL, e.g. `loom generate https://github.com/you/skills` or `--url <URL>`")?;
            commands::generate::run(&url, r#ref.as_deref(), out.as_deref())
        }
        Command::Index { out } => commands::index::run(out.as_deref()),
        Command::Agents => commands::agents::run(),
        Command::Doctor => commands::doctor::run(),
    }
}
