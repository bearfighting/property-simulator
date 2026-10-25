//! SQLite startup owned by the desktop composition root.

use std::{error::Error, path::PathBuf};

use property_simulation_data_access_sqlite::{SqliteDataAccess, sqlite_connect_options};
use sqlx::SqlitePool;
use tokio::runtime::Runtime;

pub(super) fn connect(runtime: &Runtime) -> Result<SqliteDataAccess, Box<dyn Error>> {
    let options = sqlite_connect_options(database_path());
    let pool = runtime.block_on(SqlitePool::connect_with(options))?;
    let data_access = SqliteDataAccess::new(pool);
    runtime.block_on(data_access.install_schema())?;
    Ok(data_access)
}

fn database_path() -> PathBuf {
    PathBuf::from("property-simulation.sqlite3")
}
