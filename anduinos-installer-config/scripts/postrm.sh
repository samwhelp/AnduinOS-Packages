#!/bin/sh
set -e

if [ "$1" = "remove" ] || [ "$1" = "purge" ] || [ "$1" = "abort-install" ] || [ "$1" = "disappear" ]; then
    # Delete the files we generated in postinst before undoing the
    # diversion — otherwise dpkg-divert will refuse to overwrite it.
    rm -f /usr/share/applications/ubiquity.desktop
    rm -f /usr/share/localechooser/languagelist

    dpkg-divert --remove --package anduinos-installer-config --rename \
        --divert /usr/share/localechooser/languagelist.ubuntu \
        /usr/share/localechooser/languagelist

    dpkg-divert --remove --package anduinos-installer-config --rename \
        --divert /usr/share/applications/ubiquity.desktop.ubuntu \
        /usr/share/applications/ubiquity.desktop
fi

#DEBHELPER#
exit 0
