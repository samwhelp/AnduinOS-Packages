#!/bin/sh
set -e

if [ "$1" != "configure" ]; then
    exit 0
fi

if command -v ionice >/dev/null 2>&1; then
    ionice -c3 -p "$$" 2>/dev/null || true
fi

dconf update
exit 0
