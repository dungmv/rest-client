#!/usr/bin/env bash
set -e

if [ "$EUID" -eq 0 ]; then
    BIN_DIR="/usr/local/bin"
    APP_DIR="/usr/local/share/applications"
    ICON_DIR="/usr/local/share/icons/hicolor/256x256/apps"
else
    BIN_DIR="$HOME/.local/bin"
    APP_DIR="$HOME/.local/share/applications"
    ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
fi

echo "Removing RestClient..."
rm -f "$BIN_DIR/restclient"
rm -f "$APP_DIR/restclient.desktop"
rm -f "$ICON_DIR/restclient.png"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" || true
fi

echo "RestClient uninstalled successfully."
