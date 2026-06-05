#!/bin/sh
set -e

if [ "$1" = "remove" ] || [ "$1" = "purge" ] || [ "$1" = "abort-install" ] || [ "$1" = "disappear" ]; then
    # Delete the file we generated in postinst before undoing the
    # diversion — otherwise dpkg-divert will refuse to overwrite it.
    rm -f /usr/share/applications/ubiquity.desktop

    dpkg-divert --remove --package anduinos-installer-config --rename \
        --divert /usr/share/applications/ubiquity.desktop.ubuntu \
        /usr/share/applications/ubiquity.desktop
fi

#DEBHELPER#
exit 0
