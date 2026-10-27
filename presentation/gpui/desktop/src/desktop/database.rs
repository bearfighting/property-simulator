//! SQLite startup owned by the desktop composition root.

use std::{
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

use property_simulation_data_access_sqlite::{SqliteDataAccess, sqlite_connect_options};
use sqlx::SqlitePool;
use tokio::runtime::Runtime;

pub(super) fn connect(runtime: &Runtime) -> Result<SqliteDataAccess, Box<dyn Error>> {
    let options = sqlite_connect_options(database_path()?);
    let pool = runtime.block_on(SqlitePool::connect_with(options))?;
    let data_access = SqliteDataAccess::new(pool);
    runtime.block_on(data_access.install_schema())?;
    Ok(data_access)
}

fn database_path() -> Result<PathBuf, Box<dyn Error>> {
    let data_directory = application_data_directory()?;
    fs::create_dir_all(&data_directory)?;

    let path = data_directory.join("property-simulation-config.sqlite3");
    migrate_working_directory_database(&path)?;
    Ok(path)
}

#[cfg(target_os = "windows")]
fn application_data_directory() -> Result<PathBuf, io::Error> {
    let local_app_data = env::var_os("LOCALAPPDATA")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is not set"))?;
    Ok(PathBuf::from(local_app_data).join("PropertySimulator"))
}

#[cfg(target_os = "macos")]
fn application_data_directory() -> Result<PathBuf, io::Error> {
    let home = env::var_os("HOME")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Property Simulator"))
}

#[cfg(target_os = "linux")]
fn application_data_directory() -> Result<PathBuf, io::Error> {
    if let Some(data_home) = env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        if data_home.is_absolute() {
            return Ok(data_home.join("property-simulator"));
        }
    }

    let home = env::var_os("HOME")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("property-simulator"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn application_data_directory() -> Result<PathBuf, io::Error> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "application data directory is not defined for this platform",
    ))
}

fn migrate_working_directory_database(destination: &Path) -> Result<(), io::Error> {
    if destination.exists() {
        return Ok(());
    }

    let legacy_path = env::current_dir()?.join("property-simulation-config.sqlite3");
    if legacy_path.is_file() && legacy_path != destination {
        fs::copy(legacy_path, destination)?;
    }

    Ok(())
}
