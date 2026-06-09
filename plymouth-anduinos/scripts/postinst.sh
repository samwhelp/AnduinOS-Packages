if [ "$1" = "configure" ]; then
    update-alternatives --install \
        /usr/share/plymouth/themes/default.plymouth \
        default.plymouth \
        /usr/share/plymouth/themes/anduinos/anduinos.plymouth \
        150
    update-alternatives --set \
        default.plymouth \
        /usr/share/plymouth/themes/anduinos/anduinos.plymouth || true
    if command -v dracut >/dev/null 2>&1; then
        dracut --force 2>/dev/null || true
    elif command -v update-initramfs >/dev/null 2>&1; then
        update-initramfs -u 2>/dev/null || true
    fi
fi
