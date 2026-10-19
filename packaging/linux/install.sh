#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"

if [ "$EUID" -eq 0 ]; then
    BIN_DIR="/usr/local/bin"
    APP_DIR="/usr/local/share/applications"
    ICON_DIR="/usr/local/share/icons/hicolor/256x256/apps"
else
    BIN_DIR="$HOME/.local/bin"
    APP_DIR="$HOME/.local/share/applications"
    ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

echo "Installing restclient binary to $BIN_DIR..."
install -m 755 "$DIR/restclient" "$BIN_DIR/restclient"

echo "Installing icon to $ICON_DIR..."
install -m 644 "$DIR/restclient.png" "$ICON_DIR/restclient.png"

echo "Installing desktop entry to $APP_DIR..."
sed "s|^Exec=restclient|Exec=$BIN_DIR/restclient|" "$DIR/restclient.desktop" > "$APP_DIR/restclient.desktop"
chmod 644 "$APP_DIR/restclient.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$(dirname "$(dirname "$ICON_DIR")")" || true
fi

echo "========================================="
echo " RestClient installed successfully!"
echo " Binary: $BIN_DIR/restclient"
echo " You can launch RestClient from your desktop application menu"
echo " or run 'restclient' in your terminal."
echo "========================================="
