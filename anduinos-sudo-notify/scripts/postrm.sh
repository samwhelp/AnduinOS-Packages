#!/bin/sh
set -e

PAM_FILE="/etc/pam.d/sudo"
PAM_LINE="session optional pam_exec.so /usr/libexec/anduinos/sudo-notify"

if [ "$1" = "remove" ] || [ "$1" = "purge" ]; then
    if [ -f "$PAM_FILE" ]; then
        sed -i "\|^$PAM_LINE\$|d" "$PAM_FILE"
    fi
fi

#DEBHELPER#
