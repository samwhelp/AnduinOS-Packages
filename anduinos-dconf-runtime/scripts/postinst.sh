#!/bin/sh
set -e

# anduinos-dconf-runtime owns the dpkg trigger for /etc/dconf/db.
# This postinst is invoked by dpkg:
#   - with "$1" = "triggered" when ANY package installs/removes files
#     under /etc/dconf/db/ during an apt transaction;
#   - with "$1" = "configure" when this package itself is installed/upgraded.
#
# In both cases we recompile the dconf system database.

case "$1" in
    triggered|configure)
        if command -v dconf >/dev/null 2>&1; then
            dconf update
        fi
        ;;
esac
