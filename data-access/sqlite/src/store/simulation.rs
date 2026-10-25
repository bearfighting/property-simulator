use property_simulation_data_access::dto::{
    SimulationAccountDto, SimulationDto, SimulationMonthDto, SimulationRootKeyDto,
};
use sqlx::{Row, Sqlite, Transaction};

use crate::{SqliteDataAccessError, mapping, store::SqliteDataAccess};

impl SqliteDataAccess {
    pub async fn create_simulation(
        &self,
        simulation: &SimulationDto,
    ) -> Result<(), SqliteDataAccessError> {
        sqlx::query("INSERT INTO simulations (id, name, currency) VALUES (?, ?, ?)")
            .bind(&simulation.id)
            .bind(&simulation.name)
            .bind(simulation.currency.code())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn load_simulation(
        &self,
        simulation_id: &str,
    ) -> Result<Option<SimulationDto>, SqliteDataAccessError> {
        let row = sqlx::query("SELECT id, name, currency FROM simulations WHERE id = ?")
            .bind(simulation_id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(mapping::simulation).transpose()
    }

    pub async fn save_root_key(
        &self,
        root_key: &SimulationRootKeyDto,
    ) -> Result<(), SqliteDataAccessError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO simulation_root_keys (simulation_id, root_key) VALUES (?, ?) \
             ON CONFLICT(simulation_id) DO NOTHING",
        )
        .bind(&root_key.simulation_id)
        .bind(root_key.root_key.as_slice())
        .execute(&mut *transaction)
        .await?;

        let stored_key: Vec<u8> =
            sqlx::query_scalar("SELECT root_key FROM simulation_root_keys WHERE simulation_id = ?")
                .bind(&root_key.simulation_id)
                .fetch_one(&mut *transaction)
                .await?;
        if stored_key.as_slice() != root_key.root_key {
            return Err(SqliteDataAccessError::InvalidData(
                "simulation root key is immutable",
            ));
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn load_root_key(
        &self,
        simulation_id: &str,
    ) -> Result<Option<SimulationRootKeyDto>, SqliteDataAccessError> {
        let row = sqlx::query(
            "SELECT simulation_id, root_key FROM simulation_root_keys WHERE simulation_id = ?",
        )
        .bind(simulation_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let simulation_id: String = row.try_get("simulation_id")?;
            let key: Vec<u8> = row.try_get("root_key")?;
            mapping::root_key(simulation_id, key)
        })
        .transpose()
    }

    pub async fn save_account(
        &self,
        account: &SimulationAccountDto,
    ) -> Result<(), SqliteDataAccessError> {
        sqlx::query(
            "INSERT INTO simulation_accounts (simulation_id, account_code) VALUES (?, ?) \
             ON CONFLICT(simulation_id, account_code) DO NOTHING",
        )
        .bind(&account.simulation_id)
        .bind(i64::from(account.account_code.value()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_accounts(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<SimulationAccountDto>, SqliteDataAccessError> {
        let rows = sqlx::query(
            "SELECT simulation_id, account_code FROM simulation_accounts \
             WHERE simulation_id = ? ORDER BY account_code",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                let account_code: i64 = row.try_get("account_code")?;
                let account_code = mapping::u32_value(account_code, "account_code")?;
                let account_code = property_simulation_account::AccountCode::new(account_code)
                    .map_err(|_| SqliteDataAccessError::InvalidData("account_code"))?;
                Ok(SimulationAccountDto {
                    simulation_id: row.try_get("simulation_id")?,
                    account_code,
                })
            })
            .collect()
    }

    pub async fn load_months(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<SimulationMonthDto>, SqliteDataAccessError> {
        let rows = sqlx::query(
            "SELECT simulation_id, month FROM simulation_months \
             WHERE simulation_id = ? ORDER BY month",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| mapping::month_dto(row.try_get("simulation_id")?, row.try_get("month")?))
            .collect()
    }
}

pub(super) async fn ensure_month(
    transaction: &mut Transaction<'_, Sqlite>,
    simulation_id: &str,
    month: u16,
) -> Result<(), SqliteDataAccessError> {
    sqlx::query(
        "INSERT INTO simulation_months (simulation_id, month) VALUES (?, ?) \
         ON CONFLICT(simulation_id, month) DO NOTHING",
    )
    .bind(simulation_id)
    .bind(i64::from(month))
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
