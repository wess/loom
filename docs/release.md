# Cutting a release

Loom releases are driven entirely by the crate version. Bumping `version` in
`Cargo.toml` and pushing to `main` cuts a full release — nothing else is manual.

## The flow

`.github/workflows/release.yml` runs on every push to `main` that touches
`Cargo.toml` (and on manual `workflow_dispatch`):

1. **check-version** reads `version` from `Cargo.toml` and checks whether a
   `v<version>` tag already exists on the remote. If it does, the run stops — so
   re-runs and unrelated `Cargo.toml` edits are no-ops.
2. **create-release** tags `v<version>` and opens a GitHub Release with
   auto-generated notes.
3. **build** compiles `loom` for four targets and uploads a `.tar.gz` plus a
   `.sha256` for each:
   - `aarch64-apple-darwin`, `x86_64-apple-darwin` (macOS)
   - `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` (Linux)
4. **update-homebrew** reads the four checksums back off the release and pushes
   an updated `Formula/loom.rb` to the `wess/homebrew-packages` tap, so
   `brew install wess/packages/loom` serves the new version.

## To ship a version

```sh
# 1. bump the version in Cargo.toml, e.g. version = "0.2.0"
# 2. commit and push to main
git commit -am "loom 0.2.0"
git push origin main
```

The workflow does the rest — watch it under the repo's **Actions** tab.

## Secrets

| Secret | Used by | Purpose |
| --- | --- | --- |
| `HOMEBREW_TAP_TOKEN` | `update-homebrew` | Write access to `wess/homebrew-packages` to push the formula. |

The automatic `GITHUB_TOKEN` covers tagging, the release, and asset uploads.

## Building artifacts locally

```sh
scripts/package.sh aarch64-apple-darwin
# -> dist/loom-<version>-aarch64-apple-darwin.tar.gz (+ .sha256)
```

Pass any supported target triple; omit it to build for the host.

## Publishing to crates.io (optional)

The crate is named **`loom_ai`**, not `loom` — that name belongs to an unrelated
concurrency-testing library and cannot be reused. The binary it installs is still
`loom`, because `[[bin]] name` is independent of the package name.

The crate metadata is complete, so publishing is a one-liner:

```sh
cargo publish            # dry-run first: cargo publish --dry-run
```

Once published, `cargo install loom_ai` puts a `loom` binary on the user's `PATH`.
Until then, the source route is `cargo install --git https://github.com/wess/loom`.

Publishing is deliberately kept manual and out of the release workflow. Note that
a crates.io release is **permanent** — versions can be yanked but never deleted,
and the name can never be freed.
