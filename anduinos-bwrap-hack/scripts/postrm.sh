#!/bin/sh
set -e

if [ "$1" = "remove" ] || [ "$1" = "purge" ]; then
    if [ -f /usr/bin/bwrap.real ]; then
        mv -f /usr/bin/bwrap.real /usr/bin/bwrap
    fi
fi

#DEBHELPER#
exit 0
