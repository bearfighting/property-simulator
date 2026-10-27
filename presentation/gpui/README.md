# GPUI desktop entrypoint

Run the initialization flow from the repository root:

```sh
cargo run
```

The desktop binary stores `property-simulation-config.sqlite3` in the current
user's application data directory, installs the current SQLite schema, and
then opens the GPUI initialization window. On Windows, the database is stored
under `%LOCALAPPDATA%\PropertySimulator`; on Linux it uses `$XDG_DATA_HOME` or
`~/.local/share/property-simulator`; on macOS it uses
`~/Library/Application Support/Property Simulator`. If the database exists in
the process working directory and the new location is empty, startup copies it
to the new location. The current screen uses CAD and creates Month 0 from the
opening balances and initial monthly assumptions.

On Ubuntu 24.04, GPUI Kit requires native development packages before linking:

```sh
sudo apt install gcc g++ clang libfontconfig-dev libwayland-dev \
  libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev \
  libssl-dev libzstd-dev libvulkan1
```

Other Linux distributions need their equivalent packages. See the [GPUI Kit
installation guide](https://gpui-kit.com/docs/installation/) for platform
requirements. The preview archive includes a per-user installer and desktop
icon, but these system libraries still need to be installed on the target
machine.

GitHub preview releases provide a Windows x64 ZIP and an Ubuntu 24.04 x64
`tar.gz` archive. On Linux, extract the archive and run `./install.sh` to
install the binary, application icon, and menu entry for your user. The app
then appears in the desktop applications menu and can be pinned to the taskbar.
Run `./uninstall.sh` from the extracted archive to remove those files; it leaves
the SQLite simulation data in place. To associate the icon with a local
development run started with `cargo run`, run
`sh scripts/install-dev-launcher.sh` once from the repository root.

`ui/` depends only on GPUI Kit. The `desktop/` binary owns startup, SQLite
construction, conversion from plain form data to the Application request, and
the call to `initialize_simulation`.

The desktop source is split by responsibility:

- `desktop.rs` composes the runtime, SQLite adapter, account catalogue, and UI.
- `desktop/database.rs` opens the SQLite pool and installs the schema.
- `desktop/initialization.rs` dispatches UI submissions to Tokio and invokes
  the Application use case.
- `desktop/initialization/request.rs` maps presentation strings into typed
  Application request values.
