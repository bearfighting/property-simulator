#!/bin/sh
set -eu

package_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
data_home=${XDG_DATA_HOME:-"$HOME/.local/share"}
binary_directory="$HOME/.local/bin"
icon_directory="$data_home/icons/hicolor/512x512/apps"
applications_directory="$data_home/applications"
binary_path="$binary_directory/property-simulation-desktop"
icon_path="$icon_directory/property-simulator.png"
desktop_path="$applications_directory/com.proporbit.PropertySimulator.desktop"

install -d "$binary_directory" "$icon_directory" "$applications_directory"
install -m 755 "$package_directory/property-simulation-desktop" "$binary_path"
install -m 644 "$package_directory/property-simulator.png" "$icon_path"

desktop_exec=$(printf '%s' "$binary_path" | sed 's/%/%%/g; s/\\/\\\\/g; s/"/\\"/g')
cat >"$desktop_path" <<EOF
[Desktop Entry]
Type=Application
Name=Property Simulation
Comment=Simulate property cash flow and financial outcomes
Exec="$desktop_exec"
Icon=property-simulator
Terminal=false
Categories=Office;Finance;
StartupWMClass=com.proporbit.PropertySimulator
EOF

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$applications_directory" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f "$data_home/icons/hicolor" >/dev/null 2>&1 || true
fi

printf 'Installed Property Simulation for this user. Find it in the applications menu.\n'
printf 'Executable: %s\n' "$binary_path"
printf 'Application data remains under %s/property-simulator.\n' "$data_home"
