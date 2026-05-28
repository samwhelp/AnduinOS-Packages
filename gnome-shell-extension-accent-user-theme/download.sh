#!/usr/bin/env bash
# Pre-build: downloads extension at build time. No third-party source in repo.
set -euo pipefail

UUID="accent-user-theme@brgvos"
DEPLOY_DIR="deploy/$UUID"

rm -rf "$DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR"

DOWNLOADED=false
for VER in 50 49 48 47 46 45; do
    URL="https://extensions.gnome.org/download-extension/$UUID.shell-extension.zip?shell_version=$VER"
    echo "Trying shell_version=$VER ..."
    if curl -fsSL "$URL" -o /tmp/ext.zip 2>/dev/null; then
        echo "Downloaded with shell_version=$VER"
        DOWNLOADED=true; break
    fi
done
if [ "$DOWNLOADED" = false ]; then
    echo "Trying without shell_version filter ..."
    curl -fsSL "https://extensions.gnome.org/download-extension/$UUID.shell-extension.zip" -o /tmp/ext.zip
fi

unzip -q /tmp/ext.zip -d "$DEPLOY_DIR"
rm /tmp/ext.zip

# Force GNOME Shell 50 support
jq 'if (.["shell-version"] | index("50")) then . else .["shell-version"] += ["50"] end' \
    "$DEPLOY_DIR/metadata.json" > /tmp/_meta.json
mv /tmp/_meta.json "$DEPLOY_DIR/metadata.json"
echo "Done."
