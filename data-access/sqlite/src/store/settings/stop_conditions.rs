use property_simulation_data_access::dto::StopConditionSetDto;
use property_simulation_stop_condition::StopCondition;
use sqlx::Row;

use crate::{SqliteDataAccessError, mapping, store::SqliteDataAccess};

use super::common::validate_effective_month;

impl SqliteDataAccess {
    pub async fn save_stop_condition_set(
        &self,
        settings: &StopConditionSetDto,
    ) -> Result<(), SqliteDataAccessError> {
        validate_effective_month(settings.effective_from_month)?;
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO stop_condition_sets (simulation_id, effective_from_month) VALUES (?, ?) \
             ON CONFLICT(simulation_id, effective_from_month) DO NOTHING",
        )
        .bind(&settings.simulation_id)
        .bind(i64::from(settings.effective_from_month.number()))
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "DELETE FROM stop_conditions WHERE simulation_id = ? AND effective_from_month = ?",
        )
        .bind(&settings.simulation_id)
        .bind(i64::from(settings.effective_from_month.number()))
        .execute(&mut *transaction)
        .await?;
        for condition in &settings.conditions {
            let (kind, value) = match condition {
                StopCondition::AfterMonths(months) => {
                    ("after_months", Some(i64::from(months.get())))
                }
                StopCondition::AllLoansPaidOff => ("all_loans_paid_off", None),
                StopCondition::UserStop => ("user_stop", None),
            };
            sqlx::query(
                "INSERT INTO stop_conditions (simulation_id, effective_from_month, condition, value) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(&settings.simulation_id)
            .bind(i64::from(settings.effective_from_month.number()))
            .bind(kind)
            .bind(value)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn load_stop_condition_sets(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<StopConditionSetDto>, SqliteDataAccessError> {
        let sets = sqlx::query(
            "SELECT effective_from_month FROM stop_condition_sets \
             WHERE simulation_id = ? ORDER BY effective_from_month",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        let mut result = Vec::with_capacity(sets.len());
        for set in sets {
            let effective_from_month: i64 = set.try_get("effective_from_month")?;
            let rows = sqlx::query(
                "SELECT condition, value FROM stop_conditions \
                 WHERE simulation_id = ? AND effective_from_month = ? ORDER BY id",
            )
            .bind(simulation_id)
            .bind(effective_from_month)
            .fetch_all(&self.pool)
            .await?;
            result.push(mapping::stop_condition_set(
                simulation_id.to_owned(),
                effective_from_month,
                &rows,
            )?);
        }
        Ok(result)
    }
}
