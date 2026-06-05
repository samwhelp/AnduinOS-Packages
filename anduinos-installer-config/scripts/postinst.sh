#!/bin/sh
set -e

if [ "$1" = "configure" ]; then
    dpkg-divert --add --package anduinos-installer-config --rename \
        --divert /usr/share/applications/ubiquity.desktop.ubuntu \
        /usr/share/applications/ubiquity.desktop

    sed "s%^Exec=.*%Exec=/usr/bin/anduinos-installer%" \
        /usr/share/applications/ubiquity.desktop.ubuntu \
        > /usr/share/applications/ubiquity.desktop
fi

#DEBHELPER#
exit 0
