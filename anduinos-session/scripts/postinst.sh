#!/bin/sh
set -e

if [ "$1" = "configure" ]; then
    if command -v ionice >/dev/null 2>&1; then
        ionice -c3 -p "$$" 2>/dev/null || true
    fi
    # Remove Ubuntu session overrides to prevent interference
    rm -f /usr/share/glib-2.0/schemas/10_ubuntu-session.gschema.override 2>/dev/null || true
    glib-compile-schemas /usr/share/glib-2.0/schemas/ || true
fi

#DEBHELPER#
exit 0
