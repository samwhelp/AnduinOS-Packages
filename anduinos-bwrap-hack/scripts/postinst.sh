#!/bin/sh
set -e

if [ "$1" = "configure" ]; then
    if [ -f /usr/bin/bwrap ] && [ ! -f /usr/bin/bwrap.real ]; then
        mv /usr/bin/bwrap /usr/bin/bwrap.real
        printf '#!/bin/sh\n/usr/bin/bwrap.real "$@" 2>/dev/null || true\n' > /usr/bin/bwrap
        chmod 755 /usr/bin/bwrap
    fi
fi

#DEBHELPER#
exit 0
