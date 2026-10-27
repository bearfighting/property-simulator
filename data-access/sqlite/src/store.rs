//! SQLite Data Access operations; the host creates the pool and injects it here.

mod accounting;
mod events;
mod repair_cost_instances;
mod settings;
mod simulation;

use property_simulation_money::Currency;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions};

use crate::SqliteDataAccessError;

#[derive(Clone)]
pub struct SqliteDataAccess {
    pub(super) pool: SqlitePool,
}

impl SqliteDataAccess {
    /// Wrap an application-created pool. This type never opens or owns the database file.
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create the schema using the injected pool.
    pub async fn install_schema(&self) -> Result<(), SqliteDataAccessError> {
        sqlx::raw_sql(include_str!("../schema.sql"))
            .execute(&self.pool)
            .await?;

        self.ensure_stop_conditions_configuration_marker().await?;
        Ok(())
    }

    async fn ensure_stop_conditions_configuration_marker(
        &self,
    ) -> Result<(), SqliteDataAccessError> {
        let columns = sqlx::query("PRAGMA table_info(simulations)")
            .fetch_all(&self.pool)
            .await?;
        let marker_exists = columns.iter().try_fold(false, |found, column| {
            let name: String = column.try_get("name")?;
            Ok::<_, sqlx::Error>(found || name == "stop_conditions_configured")
        })?;

        if !marker_exists {
            sqlx::query(
                "ALTER TABLE simulations ADD COLUMN stop_conditions_configured \
                 INTEGER NOT NULL DEFAULT 0 CHECK (stop_conditions_configured IN (0, 1))",
            )
            .execute(&self.pool)
            .await?;
        }

        // Keep existing non-empty configurations immutable after adding the
        // marker to a database created before this field was introduced.
        sqlx::query(
            "UPDATE simulations SET stop_conditions_configured = 1 \
             WHERE stop_conditions_configured = 0 AND EXISTS ( \
                 SELECT 1 FROM stop_conditions \
                 WHERE stop_conditions.simulation_id = simulations.id \
             )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn simulation_currency(
        &self,
        simulation_id: &str,
    ) -> Result<Currency, SqliteDataAccessError> {
        self.load_simulation(simulation_id)
            .await?
            .map(|simulation| simulation.currency)
            .ok_or(SqliteDataAccessError::InvalidData("simulation currency"))
    }
}

/// Build SQLite connection options for the host composition root.
///
/// The caller still creates the pool and injects it into [`SqliteDataAccess`].
pub fn sqlite_connect_options(path: impl AsRef<std::path::Path>) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
}
