#!/bin/sh
set -e

if [ "$1" = "configure" ]; then
    # At this point snapd has already been removed by dpkg (Conflicts in
    # the control file).  We only need to tear down leftover loop mounts
    # so that rm -rf won't hit "Device or resource busy".

    echo "Unmounting lingering snap loop devices..."
    while mount | grep -q '/snap/'; do
        mount | grep '/snap/' | awk '{print $3}' | xargs -r umount -l || break
    done

    echo "Purging snap directories..."
    rm -rf /snap
    rm -rf /var/snap
    rm -rf /var/lib/snapd
    rm -rf /var/cache/snapd
    rm -rf /usr/lib/snapd
    rm -rf /root/snap
fi

#DEBHELPER#
exit 0
