#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

FLUENT_ICON_COMMIT="8a99a6d"

rm -rf "$SCRIPT_DIR/deploy" /tmp/Fluent-icon-theme
git clone https://gitlab.aiursoft.com/mirror/fluent-icon-theme/ /tmp/Fluent-icon-theme
git -C /tmp/Fluent-icon-theme checkout "$FLUENT_ICON_COMMIT"

echo "Building Fluent icon theme (all colors)..."
mkdir -p "$SCRIPT_DIR/deploy/icons"
(
    cd /tmp/Fluent-icon-theme
    bash install.sh --all -d "$SCRIPT_DIR/deploy/icons"
)

echo "Building Fluent cursor theme..."
(
    cd /tmp/Fluent-icon-theme/cursors
    DEST_DIR="$SCRIPT_DIR/deploy/icons" bash -c '
        cp -r dist "$DEST_DIR/Fluent-cursors"
        cp -r dist-dark "$DEST_DIR/Fluent-dark-cursors"
    '
)

rm -rf /tmp/Fluent-icon-theme
echo "Done. Pre-built icon themes are in deploy/icons/."
