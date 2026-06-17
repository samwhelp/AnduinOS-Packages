#!/bin/sh
set -e

if [ "$1" != "configure" ]; then
    exit 0
fi

if command -v ionice >/dev/null 2>&1; then
    ionice -c3 -p "$$" 2>/dev/null || true
fi

SRC="/usr/share/anduinos-fluent-gtk-theme/src"
TARGET_ROOT="/usr/share/themes"
STAGING="$(mktemp -d /tmp/fluent-gtk-theme-staging.XXXXXX)"
trap 'rm -rf "$STAGING"' EXIT

echo "Building Fluent GTK theme to staging area..."
cd "$SRC"

# Build to a temp staging directory — never touch /usr/share/themes
# while GNOME Shell may be reading from it.
./install.sh --tweaks noborder round --theme all -d "$STAGING"

sync_theme_tree() {
    staged_theme="$1"
    theme_name="$(basename "$staged_theme")"
    target_theme="$TARGET_ROOT/$theme_name"
    prepared_theme="$TARGET_ROOT/.${theme_name}.new.$$"
    rollback_theme="$TARGET_ROOT/.${theme_name}.old.$$"
    had_existing_theme=0

    rm -rf "$prepared_theme" "$rollback_theme"
    cp -aT "$staged_theme" "$prepared_theme"

    if [ -e "$target_theme" ] || [ -L "$target_theme" ]; then
        mv -T "$target_theme" "$rollback_theme"
        had_existing_theme=1
    fi

    if mv -T "$prepared_theme" "$target_theme"; then
        if [ "$had_existing_theme" -eq 1 ]; then
            rm -rf "$rollback_theme"
        fi
        return 0
    fi

    rm -rf "$prepared_theme"
    if [ "$had_existing_theme" -eq 1 ] && { [ -e "$rollback_theme" ] || [ -L "$rollback_theme" ]; }; then
        mv -T "$rollback_theme" "$target_theme"
    fi
    return 1
}

echo "Syncing theme files safely..."
for staged_theme in "$STAGING"/*; do
    [ -d "$staged_theme" ] || continue
    sync_theme_tree "$staged_theme"
done

echo "Fluent GTK theme installed."
