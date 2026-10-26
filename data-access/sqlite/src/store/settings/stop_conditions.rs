use crate::{SqliteDataAccessError, mapping, store::SqliteDataAccess};
use property_simulation_data_access::dto::StopConditionsDto;
use property_simulation_stop_condition::StopCondition;

impl SqliteDataAccess {
    pub(crate) async fn save_stop_conditions(
        &self,
        settings: &StopConditionsDto,
    ) -> Result<(), SqliteDataAccessError> {
        let mut transaction = self.pool.begin().await?;
        for condition in &settings.conditions {
            let (kind, value) = match condition {
                StopCondition::AfterMonths(months) => {
                    ("after_months", Some(i64::from(months.get())))
                }
                StopCondition::AllLoansPaidOff => ("all_loans_paid_off", None),
            };
            sqlx::query(
                "INSERT INTO stop_conditions (simulation_id, condition, value) VALUES (?, ?, ?)",
            )
            .bind(&settings.simulation_id)
            .bind(kind)
            .bind(value)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub(crate) async fn load_stop_conditions(
        &self,
        simulation_id: &str,
    ) -> Result<StopConditionsDto, SqliteDataAccessError> {
        let rows = sqlx::query(
            "SELECT condition, value FROM stop_conditions WHERE simulation_id = ? ORDER BY id",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        mapping::stop_conditions(simulation_id.to_owned(), &rows)
    }
}
