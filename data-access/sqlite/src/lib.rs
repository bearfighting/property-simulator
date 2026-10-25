//! Asynchronous SQLite persistence adapter for the storage-neutral Data Access DTOs.

mod error;
mod mapping;
mod store;

pub use error::SqliteDataAccessError;
pub use store::{SqliteDataAccess, sqlite_connect_options};
