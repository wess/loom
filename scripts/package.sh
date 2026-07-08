#!/usr/bin/env bash
# Build loom (release) for one target triple and produce a .tar.gz plus a
# matching .sha256 under dist/. loom is a single CLI binary; the archive holds
# the binary alongside LICENSE and README. The version is read from Cargo.toml.
#
# The target is built with the current toolchain (the target is added if it is
# missing). macOS runners cross-build the x86_64 slice from arm natively; each
# Linux arch builds on its own native runner.
#
# Usage: scripts/package.sh [target-triple]
#   scripts/package.sh aarch64-apple-darwin
#   scripts/package.sh                        # host target
set -euo pipefail

triple="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
[ -n "$triple" ] || { echo "error: could not determine a target triple" >&2; exit 1; }

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

version="$(sed -n 's/^version = "\([0-9][^"]*\)".*/\1/p' Cargo.toml | head -1)"
[ -n "$version" ] || { echo "error: could not read version from Cargo.toml" >&2; exit 1; }
echo "[package] loom $version for $triple"

out="$root/dist"
mkdir -p "$out"

# --- build ----------------------------------------------------------------
rustup target add "$triple" >/dev/null 2>&1 || true
cargo build --release --target "$triple"

bin="target/$triple/release/loom"
[ -x "$bin" ] || { echo "error: build did not produce $bin" >&2; exit 1; }
strip "$bin" 2>/dev/null || true

# --- stage + archive ------------------------------------------------------
stem="loom-$version-$triple"
stage="$out/$stem"
rm -rf "$stage"
mkdir -p "$stage"
cp "$bin" "$stage/loom"
cp LICENSE README.md "$stage/" 2>/dev/null || true

tar -C "$out" -czf "$out/$stem.tar.gz" "$stem"
rm -rf "$stage"

# --- checksum (basename only, so it verifies from anywhere) ---------------
if command -v sha256sum >/dev/null 2>&1; then
  ( cd "$out" && sha256sum "$stem.tar.gz" > "$stem.tar.gz.sha256" )
else
  ( cd "$out" && shasum -a 256 "$stem.tar.gz" > "$stem.tar.gz.sha256" )
fi

echo "[package] -> dist/$stem.tar.gz"
cat "$out/$stem.tar.gz.sha256"
