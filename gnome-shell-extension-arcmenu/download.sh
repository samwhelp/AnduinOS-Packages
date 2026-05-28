#!/usr/bin/env bash
# Pre-build: downloads extension at build time. No third-party source in repo.
set -euo pipefail

UUID="arcmenu@arcmenu.com"
TARGET_GNOME=50
DEPLOY_DIR="deploy/$UUID"

rm -rf "$DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "$SCRIPT_DIR/../lib/resolve-gnome-ext.py" \
    "$UUID" --target "$TARGET_GNOME" --download --out "$DEPLOY_DIR"
echo "Done."
