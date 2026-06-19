#!/bin/sh
set -e

# When this package (the dconf infrastructure owner) is removed,
# recompile one last time to clean up the binary database.
case "$1" in
    remove|purge)
        if command -v dconf >/dev/null 2>&1; then
            dconf update
        fi
        ;;
esac
