#!/bin/sh
set -e

if [ "$1" = "configure" ]; then
    # Safety net: recompile the dconf database once after the full theme
    # stack is installed.  Package-level dconf files trigger dconf-cli's
    # dpkg trigger per apt transaction, but this ensures there is always a
    # fresh binary database after anduinos-theme itself is configured.
    dconf update
fi
