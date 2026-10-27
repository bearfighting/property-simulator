#!/bin/sh
set -eu

data_home=${XDG_DATA_HOME:-"$HOME/.local/share"}
binary_path="$HOME/.local/bin/property-simulation-desktop"
icon_path="$data_home/icons/hicolor/512x512/apps/property-simulator.png"
desktop_path="$data_home/applications/com.proporbit.PropertySimulator.desktop"

rm -f "$binary_path" "$icon_path" "$desktop_path"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$data_home/applications" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f "$data_home/icons/hicolor" >/dev/null 2>&1 || true
fi

printf 'Removed the application, menu entry, and icon. Simulation data was left untouched.\n'
