#!/bin/sh
set -e
if [ "$1" = "configure" ]; then
    flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo || true
fi
#DEBHELPER#
exit 0
