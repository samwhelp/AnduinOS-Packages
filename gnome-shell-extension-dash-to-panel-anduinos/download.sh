#!/usr/bin/env bash
# Pre-build: downloads dash-to-panel and applies AnduinOS customizations.
set -euo pipefail

UUID="dash-to-panel@jderose9.github.com"
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
    curl -fsSL "https://extensions.gnome.org/download-extension/$UUID.shell-extension.zip" -o /tmp/ext.zip
fi

unzip -q /tmp/ext.zip -d "$DEPLOY_DIR"
rm /tmp/ext.zip

# Patch panelPositions.js: AnduinOS default taskbar layout
PANEL_JS="$DEPLOY_DIR/panelPositions.js"
if [ -f "$PANEL_JS" ]; then
    python3 - "$PANEL_JS" <<'PYEOF'
import re, sys
path = sys.argv[1]
content = open(path).read()
new_block = ("export const defaults = [\n"
"  // AnduinOS custom default panel layout\n"
"  {\n"
"    'panel-element-positions': JSON.stringify({\n"
"      'fullWidth': [\n"
"        {'element': 'showAppsButton', 'visible': false, 'position': 'stackedTL'},\n"
"        {'element': 'activitiesButton', 'visible': false, 'position': 'stackedTL'},\n"
"        {'element': 'leftBox', 'visible': true, 'position': 'stackedTL'},\n"
"        {'element': 'taskbar', 'visible': true, 'position': 'stackedTL'},\n"
"        {'element': 'centerBox', 'visible': false, 'position': 'stackedBR'},\n"
"        {'element': 'rightBox', 'visible': true, 'position': 'stackedBR'},\n"
"        {'element': 'systemMenu', 'visible': true, 'position': 'stackedBR'},\n"
"        {'element': 'dateMenu', 'visible': true, 'position': 'stackedBR'},\n"
"        {'element': 'desktopButton', 'visible': true, 'position': 'stackedBR'},\n"
"      ]\n"
"    }),\n"
"  }\n"
"];")
result = re.sub(r'export const defaults = \[.*?\];', new_block, content, flags=re.DOTALL)
open(path, 'w').write(result)
print("Patched panelPositions.js")
PYEOF
fi

# Rename "Dash to Panel 设置" → "任务栏设置" in zh_CN locale
ZH_MO="$DEPLOY_DIR/locale/zh_CN/LC_MESSAGES/dash-to-panel.mo"
if [ -f "$ZH_MO" ]; then
    ZH_PO="${ZH_MO%.mo}.po"
    msgunfmt "$ZH_MO" -o "$ZH_PO" 2>/dev/null || true
    if [ -f "$ZH_PO" ]; then
        sed -i 's/Dash to Panel 设置/任务栏设置/g' "$ZH_PO"
        msgfmt "$ZH_PO" -o "$ZH_MO" && rm "$ZH_PO"
        echo "Patched zh_CN locale"
    fi
fi

# Force GNOME Shell 50 support
jq 'if (.["shell-version"] | index("50")) then . else .["shell-version"] += ["50"] end' \
    "$DEPLOY_DIR/metadata.json" > /tmp/_meta.json
mv /tmp/_meta.json "$DEPLOY_DIR/metadata.json"
echo "Done."
