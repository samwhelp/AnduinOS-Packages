#!/usr/bin/env bash
# Pre-build: downloads arcmenu and applies AnduinOS customizations.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
UUID="arcmenu@arcmenu.com"
DEPLOY_DIR="deploy/$UUID"

rm -rf "$DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR"

DOWNLOADED=false
for VER in 50 49 48 47 46 45; do
    URL="https://extensions.gnome.org/download-extension/$UUID.shell-extension.zip?shell_version=$VER"
    echo "Trying shell_version=$VER ..."
    if curl -fsSL "$URL" -o /tmp/ext.zip 2>/dev/null; then
        echo "Downloaded with shell_version=$VER"
        DOWNLOADED=true; break
    fi
done
if [ "$DOWNLOADED" = false ]; then
    curl -fsSL "https://extensions.gnome.org/download-extension/$UUID.shell-extension.zip" -o /tmp/ext.zip
fi

unzip -q /tmp/ext.zip -d "$DEPLOY_DIR"
rm /tmp/ext.zip

# Replace arcmenu logo with AnduinOS logo (our own asset, committed in repo)
mkdir -p "$DEPLOY_DIR/icons"
cp "$SCRIPT_DIR/anduinos-logo.svg" "$DEPLOY_DIR/icons/anduinos-logo.svg"
echo "Patched logo"

# Rename menu strings in JS (sed first, then locales reference the new strings)
if [ -f "$DEPLOY_DIR/appMenu.js" ]; then
    sed -i "s/Pin to ArcMenu/Pin to Start menu/g" "$DEPLOY_DIR/appMenu.js"
    sed -i "s/Unpin from ArcMenu/Unpin from Start menu/g" "$DEPLOY_DIR/appMenu.js"
    echo "Patched appMenu.js"
fi

# Inject translated strings for the new msgids ("Pin to Start menu" etc.)
declare -A PIN_STRINGS=(
    ["ar"]="تثبيت في قائمة ابدأ" ["be"]="Замацаваць у меню Пуск" ["bg"]="Закачи в менюто Старт"
    ["ca"]="Fixa al menú Inici" ["cs"]="Připnout do nabídky Start" ["da"]="Fastgør til startmenuen"
    ["de"]="An Startmenü anheften" ["el"]="Καρφίτσωμα στο μενού Έναρξη" ["es"]="Fijar en el menú Inicio"
    ["et"]="Kinnita algusmenüüsse" ["fi"]="Kiinnitä Käynnistä-valikkoon" ["fr"]="Épingler au menu Démarrer"
    ["he"]="הצמד לתפריט התחל" ["hi_IN"]="स्टार्ट मेनू में पिन करें" ["hu"]="Rögzítés a Start menübe"
    ["id"]="Sematkan ke menu Start" ["it"]="Aggiungi al menu Start" ["ja"]="スタートメニューに追加"
    ["ko"]="시작 메뉴에 고정" ["nb_NO"]="Fest til startmenyen" ["nl"]="Vastzetten aan Startmenu"
    ["oc"]="Afichar al menú Inici" ["pl"]="Przypnij do menu Start" ["pt_BR"]="Fixar no menu Iniciar"
    ["ru"]="Закрепить в меню Пуск" ["si"]="ආරම්භ මෙනුවට අමුණන්න" ["sk"]="Pripnúť do ponuky Štart"
    ["sr"]="Закачи у мени Старт" ["sr@latin"]="Zakači u meni Start" ["sv"]="Fäst i startmenyn"
    ["szl"]="Przipnij do menu Start" ["tr"]="Başlat menüsüne sabitle" ["uk"]="Закріпити в меню Пуск"
    ["zh_CN"]="固定到开始菜单" ["zh_TW"]="固定到開始功能表"
)
declare -A UNPIN_STRINGS=(
    ["ar"]="إلغاء التثبيت من قائمة ابدأ" ["be"]="Адмацаваць з меню Пуск" ["bg"]="Откачи от менюто Старт"
    ["ca"]="Desfixa del menú Inici" ["cs"]="Odepnout z nabídky Start" ["da"]="Frigør fra startmenuen"
    ["de"]="Vom Startmenü lösen" ["el"]="Ξεκαρφίτσωμα από το μενού Έναρξη" ["es"]="Desfijar del menú Inicio"
    ["et"]="Eemalda algusmenüüst" ["fi"]="Irrota Käynnistä-valikosta" ["fr"]="Désépingler du menu Démarrer"
    ["he"]="הסר הצמדה מתפריט התחל" ["hi_IN"]="स्टार्ट मेनू से अनपिन करें" ["hu"]="Eltávolítás a Start menüből"
    ["id"]="Lepas dari menu Start" ["it"]="Rimuovi dal menu Start" ["ja"]="スタートメニューから削除"
    ["ko"]="시작 메뉴에서 고정 해제" ["nb_NO"]="Løsne fra startmenyen" ["nl"]="Losmaken van Startmenu"
    ["oc"]="Retirar del menú Inici" ["pl"]="Odepnij od menu Start" ["pt_BR"]="Desafixar do menu Iniciar"
    ["ru"]="Открепить от меню Пуск" ["si"]="ආරම්භ මෙනුවෙන් ගලවන්න" ["sk"]="Odopnúť z ponuky Štart"
    ["sr"]="Откачи из менија Старт" ["sr@latin"]="Otkači iz menija Start" ["sv"]="Lossa från startmenyn"
    ["szl"]="Odpnij od menu Start" ["tr"]="Başlat menüsünden çıkar" ["uk"]="Відкріпити з меню Пуск"
    ["zh_CN"]="从开始菜单取消固定" ["zh_TW"]="從開始功能表取消固定"
)

LOCALE_DIR="$DEPLOY_DIR/locale"
PATCHED=0
for lang_dir in "$LOCALE_DIR"/*/; do
    lang=$(basename "$lang_dir")
    mo_file="$lang_dir/LC_MESSAGES/arcmenu.mo"
    [ -f "$mo_file" ] || continue
    [ -n "${PIN_STRINGS[$lang]+isset}" ] || continue

    po_file="${mo_file%.mo}.po"
    msgunfmt "$mo_file" -o "$po_file" 2>/dev/null || continue
    cat >> "$po_file" << POEOF

msgid "Pin to Start menu"
msgstr "${PIN_STRINGS[$lang]}"

msgid "Unpin from Start menu"
msgstr "${UNPIN_STRINGS[$lang]}"
POEOF
    msgfmt "$po_file" -o "$mo_file" && rm "$po_file"
    PATCHED=$((PATCHED + 1))
done
echo "Patched $PATCHED locale files"

# Force GNOME Shell 50 support
jq 'if (.["shell-version"] | index("50")) then . else .["shell-version"] += ["50"] end' \
    "$DEPLOY_DIR/metadata.json" > /tmp/_meta.json
mv /tmp/_meta.json "$DEPLOY_DIR/metadata.json"
echo "Done."
