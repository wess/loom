# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Loom is a package manager for AI skills — think Homebrew, but for the reusable `SKILL.md` bundles that agents like Claude Code, Codex, and Cursor load. A **manifest** (`skills/<name>.yml`) is a recipe that describes *where to fetch a skill and how to install it*; it never contains the skill payload. The `loom` CLI resolves a manifest, fetches the payload from its `source`, and copies it into an agent's skills directory.

## Commands

```sh
cargo build                       # debug build
cargo build --release             # release build -> target/release/loom
cargo test                        # run all unit tests
cargo test manifest::tests        # run one module's tests
cargo test yaml_roundtrips        # run a single test by name
cargo run -- <subcommand> ...     # run the CLI in dev
```

Running the CLI touches real state, so isolate it with `LOOM_HOME`:

```sh
LOOM_HOME=/tmp/loomtest cargo run -- lint       # lint every manifest in skills/
LOOM_HOME=/tmp/loomtest cargo run -- index --out docs/skills.json   # rebuild website index
```

`loom` shells out to the system `git`, so `git` must be on `PATH`. Network is required for `install`/`test`/`generate`/`update`.

## Architecture

The binary is organized by domain, one small module per concern (each is a folder with `mod.rs`). Data flows: **CLI → command handler → domain modules**.

- `cli/` — clap definitions and `run()`, which parses args and dispatches to a `commands::*` handler. Adding a subcommand means editing the `Command` enum here *and* adding a handler file.
- `commands/` — one file per subcommand. Handlers are thin: they load `Config`, resolve a manifest, and call into domain modules. `commands/mod.rs` has two shared helpers: `open_repo()` and `resolve_manifest()` (the latter accepts either a repo skill name or a local `.yml` path). Two commands are heavier: `init` is an interactive `dialoguer` wizard that bails unless stdin `IsTerminal` (so it can't be tested non-interactively), and `publish` orchestrates `gh`/`git` to fork+branch+PR — it **defaults to a dry run** and only mutates with `--execute`.
- `manifest/` — the `Manifest` type and the `<name>.yml` contract. **This is the central data structure everything depends on.** Note the `source.ref_` field is serde-renamed to `ref` (YAML keyword). `lint()` returns `Vec<Problem>` with a `Severity` (Error vs Warning); `validate()` only fails on `Error`, so advisory nits (missing authors, long description, unpinned git ref) never block an install.
- `repo/` — reads and searches the `skills/` folder (the manifest repository / "tap").
- `fetch/` — materializes a `Source` into a scratch dir: git via shallow `git clone` (falls back to full clone + checkout for commit SHAs), archive via HTTPS download + sha256 verify + tar/gz unpack (a single wrapping top-level dir is auto-flattened). Returns a `Payload` rooted at the resolved `subdir`.
- `install/` — copies a payload into `<agent skills dir>/<name>/`, verifies the entry file, and maintains the install registry (`state.json`). Handles install/uninstall/upgrade.
- `generate/` — clones a repo, finds every folder with a `SKILL.md`, reads its front matter, and emits a manifest per skill (detecting license and repo owner).
- `site/` — builds the JSON search index the website reads.
- `config/`, `paths/`, `output/` — agent registry + settings, filesystem locations, terminal formatting.

### State on disk

Everything Loom owns lives under one prefix, `~/.loom` (override with `LOOM_HOME`): `config.json` (agents + repo settings), `state.json` (installed-skill registry), `cache/` (clones, downloads, scratch builds). Agents map an id (`claude-code`, `codex`, `cursor`) to a skills directory; the manifest's `compatibility` list is advisory only — install target comes from `--agent`/`default_agent`, not the manifest.

### The website

`docs/` is a dependency-free static GitHub Pages site (no build step). Its search runs client-side over `docs/skills.json`, which is generated from `skills/` by `loom index`. **Contributors don't regenerate the index by hand:** the `reindex` GitHub Actions workflow rebuilds `docs/skills.json` and commits it back whenever a manifest lands on `main` (`.github/workflows/reindex.yml`). PR CI only checks that the index *builds* cleanly, not that it's committed. Regenerate locally (`loom index --out docs/skills.json`) only if you want to preview the site before merge. The site is served as a **project page under `https://wess.io/loom/`**, so every internal link/asset reference in `docs/` must be relative (no leading `/`) or it 404s. `docs/demo.svg` is a hand-authored, self-contained animated terminal (no external tooling); `docs/demo.tape` re-records it as a GIF via `vhs` if ever needed.

## Conventions

- File names are lowercase with no spaces, dashes, or underscores (repo owner's standard). Rust modules are folders with `mod.rs`; website assets are short single words (`style.css`, `app.js`, `search.js`).
- Errors propagate with `anyhow` (`?` + `.context()`); `main` prints the chain and returns a failure exit code. Terminal output goes through `output/` helpers (`step`/`ok`/`warn`/`error`/`detail`) rather than raw `println!`.
- Unit tests are inline `#[cfg(test)] mod tests` in the module they cover (see `manifest/` and `generate/`).
