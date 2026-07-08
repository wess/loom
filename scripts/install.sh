#!/usr/bin/env sh
# loom installer. Detects your OS and architecture, downloads the matching
# release tarball from GitHub, verifies its sha256, and installs the `loom`
# binary into a bin directory on your PATH.
#
#   curl -fsSL https://raw.githubusercontent.com/wess/loom/main/scripts/install.sh | sh
#
# Environment:
#   LOOM_VERSION   version to install (default: latest release)
#   LOOM_BIN_DIR   install directory (default: /usr/local/bin if writable, else ~/.local/bin)
set -eu

repo="wess/loom"

say() { printf '%s\n' "$*"; }
err() { printf 'error: %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || err "missing required tool: $1"; }

need curl
need tar
need uname

# --- pick the release asset for this platform -----------------------------
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Darwin) os_part="apple-darwin" ;;
  Linux)  os_part="unknown-linux-gnu" ;;
  *) err "unsupported OS: $os (use Homebrew or grab a release manually)" ;;
esac
case "$arch" in
  x86_64 | amd64)  arch_part="x86_64" ;;
  arm64 | aarch64) arch_part="aarch64" ;;
  *) err "unsupported architecture: $arch" ;;
esac
triple="$arch_part-$os_part"

# --- resolve the version --------------------------------------------------
version="${LOOM_VERSION:-}"
if [ -z "$version" ]; then
  version="$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" \
    | sed -n 's/.*"tag_name": *"v\{0,1\}\([^"]*\)".*/\1/p' | head -1)"
  [ -n "$version" ] || err "could not determine the latest version; set LOOM_VERSION"
fi
version="${version#v}"

stem="loom-$version-$triple"
url="https://github.com/$repo/releases/download/v$version/$stem.tar.gz"
say "Downloading loom $version ($triple)..."

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL -o "$tmp/loom.tar.gz" "$url" || err "download failed: $url"

# --- verify the checksum if the .sha256 asset is present ------------------
if curl -fsSL -o "$tmp/loom.tar.gz.sha256" "$url.sha256" 2>/dev/null; then
  expected="$(cut -d' ' -f1 "$tmp/loom.tar.gz.sha256")"
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$tmp/loom.tar.gz" | cut -d' ' -f1)"
  else
    actual="$(shasum -a 256 "$tmp/loom.tar.gz" | cut -d' ' -f1)"
  fi
  [ "$expected" = "$actual" ] || err "checksum mismatch (expected $expected, got $actual)"
  say "Checksum verified."
fi

# --- unpack + install -----------------------------------------------------
tar -xzf "$tmp/loom.tar.gz" -C "$tmp"
binpath="$(find "$tmp" -type f -name loom | head -1)"
[ -n "$binpath" ] || err "loom binary not found in the archive"

bindir="${LOOM_BIN_DIR:-}"
if [ -z "$bindir" ]; then
  if [ -w /usr/local/bin ]; then
    bindir="/usr/local/bin"
  else
    bindir="$HOME/.local/bin"
  fi
fi
mkdir -p "$bindir"
if command -v install >/dev/null 2>&1; then
  install -m 0755 "$binpath" "$bindir/loom"
else
  cp "$binpath" "$bindir/loom" && chmod 0755 "$bindir/loom"
fi

say "Installed loom to $bindir/loom"
case ":$PATH:" in
  *":$bindir:"*) : ;;
  *) say "Note: $bindir is not on your PATH. Add it, e.g.:"
     say "  export PATH=\"$bindir:\$PATH\"" ;;
esac
"$bindir/loom" --version 2>/dev/null || true
