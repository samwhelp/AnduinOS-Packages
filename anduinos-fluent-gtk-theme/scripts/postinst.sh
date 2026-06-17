#!/bin/sh
set -e

SRC="/usr/share/anduinos-fluent-gtk-theme/src"
STAGING="/tmp/fluent-gtk-theme-staging"

echo "Building Fluent GTK theme to staging area..."
cd "$SRC"

# Build to a temp staging directory — never touch /usr/share/themes
# while GNOME Shell may be reading from it.
rm -rf "$STAGING"
mkdir -p "$STAGING"
./install.sh --tweaks noborder round --theme all -d "$STAGING"

# Atomically sync each file: write to .new, then mv into place.
# GNOME Shell always sees either the old complete file or the new complete file.
echo "Syncing theme files atomically..."
find "$STAGING" -type f -exec sh -c '
    src="$1"
    target="/usr/share/${src#'"$STAGING"'/}"
    mkdir -p "$(dirname "$target")"
    cp -a "$src" "$target.new"
    mv "$target.new" "$target"
' _ {} \;
rm -rf "$STAGING"

echo "Fluent GTK theme installed."
