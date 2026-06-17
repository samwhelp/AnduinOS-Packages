#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

FLUENT_GTK_COMMIT="9fc5291"

rm -rf "$SCRIPT_DIR/deploy" /tmp/Fluent-gtk-theme
git clone https://gitlab.aiursoft.com/mirror/fluent-gtk-theme/ /tmp/Fluent-gtk-theme
git -C /tmp/Fluent-gtk-theme checkout "$FLUENT_GTK_COMMIT"

echo "Building Fluent GTK theme (all colors, noborder + round)..."
mkdir -p "$SCRIPT_DIR/deploy/themes"
(
    cd /tmp/Fluent-gtk-theme
    # sassc is required at build time; GS_VERSION defaults to 48-0 when
    # gnome-shell is not installed on the build server.
    ./install.sh --tweaks noborder round --theme all -d "$SCRIPT_DIR/deploy/themes"
)

rm -rf /tmp/Fluent-gtk-theme
echo "Done. Pre-built GTK themes are in deploy/themes/."
