#!/bin/sh
set -eu

if [ "$(uname -s)" != Linux ]; then
    printf 'This launcher setup is for Linux desktops.\n' >&2
    exit 1
fi

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_directory/.." && pwd)
data_home=${XDG_DATA_HOME:-"$HOME/.local/share"}
binary_directory="$HOME/.local/bin"
applications_directory="$data_home/applications"
icon_directory="$data_home/icons/hicolor/512x512/apps"
launcher_path="$binary_directory/property-simulator-dev"
desktop_path="$applications_directory/com.proporbit.PropertySimulator.desktop"

install -d "$binary_directory" "$applications_directory" "$icon_directory"
cat >"$launcher_path" <<EOF
#!/bin/sh
exec cargo run --manifest-path "$repository_root/Cargo.toml" --package property-simulation-desktop "\$@"
EOF
chmod 755 "$launcher_path"
install -m 644 "$repository_root/assets/app-icons/prop-orbit-512.png" \
    "$icon_directory/property-simulator.png"

desktop_exec=$(printf '%s' "$launcher_path" | sed 's/%/%%/g; s/\\/\\\\/g; s/"/\\"/g')
cat >"$desktop_path" <<EOF
[Desktop Entry]
Type=Application
Name=Property Simulation (development)
Comment=Run the local Property Simulation build
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

printf 'Registered the local app icon and launcher. You can keep running cargo run from the repository.\n'
