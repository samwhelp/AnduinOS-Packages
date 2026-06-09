if [ "$1" = "remove" ] || [ "$1" = "deconfigure" ]; then
    update-alternatives --remove \
        default.plymouth \
        /usr/share/plymouth/themes/anduinos/anduinos.plymouth || true
    if command -v dracut >/dev/null 2>&1; then
        dracut --force 2>/dev/null || true
    elif command -v update-initramfs >/dev/null 2>&1; then
        update-initramfs -u 2>/dev/null || true
    fi
fi
