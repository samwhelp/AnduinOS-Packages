#!/bin/sh
set -e

SRC="/usr/share/anduinos-fluent-icon-theme/src"

echo "Installing Fluent icon theme..."
cd "$SRC"
./install.sh --all -d /usr/share/icons

echo "Installing Fluent cursor theme..."
cd "$SRC/cursors"
./install.sh

# install.sh already runs gtk-update-icon-cache per variant.
# Running it again with -f -t across all Fluent* dirs (including
# cursors without index.theme) can corrupt the cache.
echo "Fluent icon theme installed."
