#!/bin/bash
# Install shan-shui and make it the Omarchy screensaver.
#
#   ./install.sh           download the latest prebuilt release (no Rust needed)
#   ./install.sh --build   build this checkout from source instead (needs Rust)
#
# Also works without cloning:
#   curl -fsSL https://raw.githubusercontent.com/yenst/shan-shui-omarchy/main/install.sh | bash
set -euo pipefail

REPO=yenst/shan-shui-omarchy
ASSET=shan-shui-x86_64-linux.tar.gz
DEST="$HOME/.local/share/shan-shui/bin"
BEGIN="# >>> shan-shui screensaver >>>"
END="# <<< shan-shui screensaver <<<"

# The checkout this script lives in, if any (empty when piped from curl)
src=""
if [[ -n ${BASH_SOURCE[0]:-} && -f "$(dirname "${BASH_SOURCE[0]}")/Cargo.toml" ]]; then
  src=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

install_release() {
  if [[ $(uname -m) != x86_64 ]]; then
    echo "Prebuilt releases are x86_64 only." >&2
    return 1
  fi
  local url="https://github.com/$REPO/releases/latest/download"
  echo "Downloading the latest release..."
  curl -fsSL "$url/$ASSET" -o "$tmp/$ASSET" || return 1
  curl -fsSL "$url/$ASSET.sha256" -o "$tmp/$ASSET.sha256" || return 1
  (cd "$tmp" && sha256sum --check --quiet "$ASSET.sha256") || {
    echo "Checksum mismatch, refusing to install." >&2
    return 1
  }
  tar -xzf "$tmp/$ASSET" -C "$tmp" || return 1
  mkdir -p "$DEST"
  install -m755 "$tmp/shan-shui/shan-shui" "$tmp/shan-shui/omarchy-launch-screensaver" "$DEST/"
}

build_source() {
  if [[ -z $src ]]; then
    echo "To build from source: git clone https://github.com/$REPO && cd shan-shui-omarchy && ./install.sh --build" >&2
    return 1
  fi
  # rustup installs cargo here; load it if this shell doesn't have it yet
  if ! command -v cargo >/dev/null && [[ -f $HOME/.cargo/env ]]; then
    . "$HOME/.cargo/env"
  fi
  if ! command -v cargo >/dev/null; then
    echo "Rust is not installed. Run:  omarchy install dev-env rust" >&2
    echo "then open a new terminal and run ./install.sh --build again." >&2
    return 1
  fi
  echo "Building (takes a minute the first time)..."
  (cd "$src" && cargo build --release --locked) || return 1
  local target_dir
  target_dir=$(cd "$src" && cargo metadata --format-version 1 --no-deps | jq -r .target_directory)
  mkdir -p "$DEST"
  install -m755 "$target_dir/release/shan-shui" "$src/scripts/omarchy-launch-screensaver" "$DEST/"
}

if [[ ${1:-} == --build ]]; then
  build_source
elif ! install_release; then
  echo "Download failed, building from source instead." >&2
  build_source
fi

# Put $DEST first on PATH for login shells. Omarchy's idle timer and menu run
# `bash -lc omarchy-launch-screensaver`, so this is what makes them pick ours.
# The block must sit above .bashrc's "not interactive, return" line.
rc="$HOME/.bashrc"
touch "$rc"
if ! grep -qF "$BEGIN" "$rc"; then
  block="$BEGIN
export PATH=\"\$HOME/.local/share/shan-shui/bin:\$PATH\"
$END"
  if grep -q "env-bootstrap" "$rc"; then
    awk -v block="$block" '{ print } /env-bootstrap/ && !done { print block; done = 1 }' "$rc" >"$tmp/bashrc"
  else
    { printf '%s\n' "$block"; cat "$rc"; } >"$tmp/bashrc"
  fi
  cp "$rc" "$rc.bak.shan-shui"
  cat "$tmp/bashrc" >"$rc"
  echo "Added shan-shui to PATH in ~/.bashrc (backup: ~/.bashrc.bak.shan-shui)"
fi

if bash -lc 'command -v omarchy-launch-screensaver' | grep -q shan-shui; then
  echo "Installed $("$DEST/shan-shui" --version). Try it now:  omarchy-launch-screensaver force"
else
  echo "Installed, but omarchy-launch-screensaver doesn't resolve to shan-shui yet." >&2
  echo "Something in your shell config puts another directory first on PATH." >&2
  exit 1
fi
