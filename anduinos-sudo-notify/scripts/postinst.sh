#!/bin/sh
set -e

PAM_FILE="/etc/pam.d/sudo"
PAM_LINE="session optional pam_exec.so /usr/libexec/anduinos/sudo-notify"

if [ "$1" = "configure" ]; then
    if [ -f "$PAM_FILE" ]; then
        if ! grep -qF "$PAM_LINE" "$PAM_FILE"; then
            # Insert before the first @include line
            sed -i "0,/^@include/{s/^@include/$PAM_LINE\n@include/}" "$PAM_FILE"
        fi
    fi
fi

#DEBHELPER#
