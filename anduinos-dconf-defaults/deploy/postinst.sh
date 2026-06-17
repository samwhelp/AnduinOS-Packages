#!/bin/sh
set -e

if [ "$1" != "configure" ]; then
    exit 0
fi

if command -v ionice >/dev/null 2>&1; then
    ionice -c3 -p "$$" 2>/dev/null || true
fi

glib-compile-schemas /usr/share/glib-2.0/schemas/ || true
dconf update || true
exit 0
