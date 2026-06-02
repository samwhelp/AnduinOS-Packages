#!/bin/sh
set -e

ICON_DIR="/usr/share/icons"

# Restore symlinks from manifest (dpkg resolves symlinks at build time)
MANIFEST="$ICON_DIR/Fluent-symlinks.txt"
if [ -f "$MANIFEST" ]; then
    echo "Restoring Fluent icon symlinks..."
    restored=0
    while IFS="$(printf '\t')" read -r link target; do
        linkpath="$ICON_DIR/$link"
        linkdir="$(dirname "$linkpath")"
        [ -d "$linkdir" ] || mkdir -p "$linkdir"
        ln -sf "$target" "$linkpath"
        restored=$((restored + 1))
    done < "$MANIFEST"
    echo "Restored $restored symlinks — removing manifest"
    rm -f "$MANIFEST"
fi

# Update icon caches
for theme in "$ICON_DIR"/Fluent*; do
    if [ -d "$theme" ]; then
        gtk-update-icon-cache -f -t "$theme" 2>/dev/null || true
    fi
done
