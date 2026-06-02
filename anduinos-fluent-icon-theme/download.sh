#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

FLUENT_ICON_COMMIT="8a99a6d"   # pinned for supply-chain safety

rm -rf "$SCRIPT_DIR/deploy" /tmp/Fluent-icon-theme
git clone https://github.com/vinceliuice/Fluent-icon-theme.git /tmp/Fluent-icon-theme
git -C /tmp/Fluent-icon-theme checkout "$FLUENT_ICON_COMMIT"

STAGING="$SCRIPT_DIR/deploy/icons"

echo "Installing icon theme to staging..."
cd /tmp/Fluent-icon-theme
./install.sh --all -d "$STAGING"

echo "Installing cursor theme..."
cd /tmp/Fluent-icon-theme/cursors
cp -r dist "$STAGING/Fluent-cursors"
cp -r dist-dark "$STAGING/Fluent-dark-cursors"

# ── Symlink manifest — dpkg resolves symlinks → 3.4 GB. Ship real files only,
#   record symlinks, restore in postinst. ────────────────────────────────
MANIFEST="$SCRIPT_DIR/deploy/symlinks.txt"
rm -f "$MANIFEST"

echo "Recording symlinks..."
find "$STAGING" -type l -printf "%P\t%l\n" > "$MANIFEST"

count=$(wc -l < "$MANIFEST")
echo "  $count symlinks recorded"

# Delete symlinks — only real files go into the .deb
echo "Stripping symlinks..."
find "$STAGING" -type l -delete

# Remove broken symlinks that were already dead
find "$STAGING" -xtype l -delete 2>/dev/null || true

rm -rf /tmp/Fluent-icon-theme
echo "Done."
