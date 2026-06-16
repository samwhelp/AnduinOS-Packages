#!/usr/bin/env bash
# Pre-build: downloads Desktop Icons NG (DING) for each supported suite,
# then applies AnduinOS customizations.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/gnome-versions.sh"

UUID="ding@rastersoft.com"

for SUITE in "${!GNOME_TARGETS[@]}"; do
    TARGET=${GNOME_TARGETS[$SUITE]}
    DEPLOY_DIR="deploy/$SUITE/$UUID"

    rm -rf "$DEPLOY_DIR"
    mkdir -p "$DEPLOY_DIR"

    echo "[$SUITE] Resolving $UUID for GNOME $TARGET..."
    python3 "$SCRIPT_DIR/../lib/resolve-gnome-ext.py" "$UUID" --target "$TARGET" --download --out "$DEPLOY_DIR"

    # ── AnduinOS customizations ───────────────────────────────────────────
    echo "[$SUITE] Patching desktopManager.js: Desktop Icons Settings → AnduinOS Appearance Settings"

    sed -i "s/label: _('Desktop Icons Settings')/label: _('AnduinOS Appearance Settings')/" \
        "$DEPLOY_DIR/app/desktopManager.js"

    sed -i 's/this._settingsMenuItem.connect("activate", () => Prefs.showPreferences());/this._settingsMenuItem.connect("activate", () => { GLib.spawn_command_line_async('\''anduinos-appearance'\''); });/' \
        "$DEPLOY_DIR/app/desktopManager.js"

    echo "[$SUITE] Patch applied successfully."
done

echo "Done."
