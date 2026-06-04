#!/bin/sh
set -e
if [ "$1" = "configure" ]; then
    fc-cache -f
fi
#DEBHELPER#
exit 0
