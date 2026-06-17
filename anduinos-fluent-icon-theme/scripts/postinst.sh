#!/bin/sh
set -e

if [ "$1" != "configure" ]; then
    exit 0
fi

if command -v ionice >/dev/null 2>&1; then
    ionice -c3 -p "$$" 2>/dev/null || true
fi

ARCHIVE="/usr/share/anduinos-fluent-icon-theme/fluent-icon-theme.tar.gz"
WORKDIR="$(mktemp -d /tmp/anduinos-fluent-icon-theme.XXXXXX)"
trap 'rm -rf "$WORKDIR"' EXIT

echo "Extracting Fluent icon theme sources..."
tar -xzf "$ARCHIVE" -C "$WORKDIR"

SRC="$WORKDIR/Fluent-icon-theme"

echo "Installing Fluent icon theme..."
(
    cd "$SRC" && \
    ./install.sh --all -d /usr/share/icons
)

echo "Installing Fluent cursor theme..."
(
    cd "$SRC/cursors" && \
    ./install.sh
)

# Upstream install.sh already rebuilds the cache for each Fluent theme it installs.
echo "Fluent icon theme installed."
