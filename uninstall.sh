#!/bin/bash
# Remove shan-shui and restore the stock Omarchy screensaver.
set -euo pipefail
rm -rf "$HOME/.local/share/shan-shui"
if [[ -f $HOME/.bashrc ]]; then
  sed -i '/# >>> shan-shui screensaver >>>/,/# <<< shan-shui screensaver <<</d' "$HOME/.bashrc"
fi
echo "Removed. The stock Omarchy screensaver is back."
