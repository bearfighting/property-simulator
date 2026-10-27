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
requirements. The Linux preview archive contains the executable, but these
system libraries still need to be installed on the target machine.

GitHub preview releases provide a Windows x64 ZIP and an Ubuntu 24.04 x64
`tar.gz` archive. Extract the archive, then run `property-simulation-desktop`
from a terminal on Linux (you may need to grant it execute permission with
`chmod +x property-simulation-desktop`).

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
