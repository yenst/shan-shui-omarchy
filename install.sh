#!/bin/bash
# Build shan-shui and make it the Omarchy screensaver.
set -euo pipefail
cd "$(dirname "$0")"

DEST="$HOME/.local/share/shan-shui/bin"
BEGIN="# >>> shan-shui screensaver >>>"
END="# <<< shan-shui screensaver <<<"

# rustup installs cargo here; load it if this shell doesn't have it yet
if ! command -v cargo >/dev/null && [[ -f $HOME/.cargo/env ]]; then
  . "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null; then
  echo "Rust is not installed. Run:  omarchy install dev-env rust" >&2
  echo "then open a new terminal and run ./install.sh again." >&2
  exit 1
fi

echo "Building (takes a minute the first time)..."
cargo build --release --locked

mkdir -p "$DEST"
target_dir=$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)
install -m755 "$target_dir/release/shan-shui" "$DEST/shan-shui"
install -m755 scripts/omarchy-launch-screensaver "$DEST/omarchy-launch-screensaver"

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
    awk -v block="$block" '{ print } /env-bootstrap/ && !done { print block; done = 1 }' "$rc" >"$rc.shan-shui.tmp"
  else
    { printf '%s\n' "$block"; cat "$rc"; } >"$rc.shan-shui.tmp"
  fi
  cp "$rc" "$rc.bak.shan-shui"
  mv "$rc.shan-shui.tmp" "$rc"
  echo "Added shan-shui to PATH in ~/.bashrc (backup: ~/.bashrc.bak.shan-shui)"
fi

if bash -lc 'command -v omarchy-launch-screensaver' | grep -q shan-shui; then
  echo "Installed. Try it now:  omarchy-launch-screensaver force"
else
  echo "Installed, but omarchy-launch-screensaver doesn't resolve to shan-shui yet." >&2
  echo "Something in your shell config puts another directory first on PATH." >&2
  exit 1
fi
