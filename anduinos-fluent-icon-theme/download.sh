#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf "$SCRIPT_DIR/deploy" /tmp/Fluent-icon-theme
git clone --depth 1 https://github.com/vinceliuice/Fluent-icon-theme.git /tmp/Fluent-icon-theme

echo "Installing icon theme to staging..."
cd /tmp/Fluent-icon-theme
./install.sh --all -d "$SCRIPT_DIR/deploy/icons"

echo "Installing cursor theme..."
cd cursors && ./install.sh -d "$SCRIPT_DIR/deploy/icons"

rm -rf /tmp/Fluent-icon-theme
echo "Done."
