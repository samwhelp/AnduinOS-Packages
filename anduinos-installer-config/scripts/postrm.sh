#!/bin/sh
set -e

if [ "$1" = "remove" ] || [ "$1" = "purge" ] || [ "$1" = "abort-install" ] || [ "$1" = "disappear" ]; then
    # Delete the files we generated in postinst before undoing the
    # diversion — otherwise dpkg-divert will refuse to overwrite it.
    rm -f /usr/share/applications/ubiquity.desktop
    rm -f /usr/lib/ubiquity/localechooser/languagelist
    rm -f /usr/lib/ubiquity/localechooser/languagelist.data.gz

    dpkg-divert --remove --package anduinos-installer-config --rename \
        --divert /usr/lib/ubiquity/localechooser/languagelist.data.gz.ubuntu \
        /usr/lib/ubiquity/localechooser/languagelist.data.gz

    dpkg-divert --remove --package anduinos-installer-config --rename \
        --divert /usr/lib/ubiquity/localechooser/languagelist.ubuntu \
        /usr/lib/ubiquity/localechooser/languagelist

    dpkg-divert --remove --package anduinos-installer-config --rename \
        --divert /usr/share/applications/ubiquity.desktop.ubuntu \
        /usr/share/applications/ubiquity.desktop
fi

#DEBHELPER#
exit 0
