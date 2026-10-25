use crate::{SqliteDataAccessError, mapping, store::SqliteDataAccess};
use property_simulation_data_access::dto::{RepairCostSettingsDto, RepairOccurrenceSettingsDto};

use super::common::{bind_money, validate_currency, validate_effective_month};

impl SqliteDataAccess {
    pub(crate) async fn save_repair_occurrence_settings(
        &self,
        settings: &RepairOccurrenceSettingsDto,
    ) -> Result<(), SqliteDataAccessError> {
        validate_effective_month(settings.effective_from_month)?;
        sqlx::query(
            "INSERT INTO repair_occurrence_settings (simulation_id, effective_from_month, enabled) \
             VALUES (?, ?, ?) ON CONFLICT(simulation_id, effective_from_month) DO UPDATE SET \
             enabled = excluded.enabled",
        )
        .bind(&settings.simulation_id)
        .bind(i64::from(settings.effective_from_month.number()))
        .bind(settings.enabled.map(i64::from))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn load_repair_occurrence_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RepairOccurrenceSettingsDto>, SqliteDataAccessError> {
        let rows = sqlx::query(
            "SELECT * FROM repair_occurrence_settings WHERE simulation_id = ? ORDER BY effective_from_month",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(mapping::repair_occurrence_settings)
            .collect()
    }

    pub(crate) async fn save_repair_cost_settings(
        &self,
        settings: &RepairCostSettingsDto,
    ) -> Result<(), SqliteDataAccessError> {
        validate_effective_month(settings.effective_from_month)?;
        let currency = self.simulation_currency(&settings.simulation_id).await?;
        if let Some(parameters) = settings.parameters {
            validate_currency(parameters.maximum_amount(), currency)?;
        }
        let (coefficient, scale, delay) =
            settings
                .parameters
                .map_or((None, None, None), |parameters| {
                    let (coefficient, scale) = bind_money(parameters.maximum_amount());
                    (
                        Some(coefficient),
                        Some(scale),
                        Some(i64::from(parameters.delay_months().get())),
                    )
                });
        sqlx::query(
            "INSERT INTO repair_cost_settings \
             (simulation_id, effective_from_month, maximum_amount_coefficient, maximum_amount_scale, delay_months) \
             VALUES (?, ?, ?, ?, ?) ON CONFLICT(simulation_id, effective_from_month) DO UPDATE SET \
             maximum_amount_coefficient = excluded.maximum_amount_coefficient, \
             maximum_amount_scale = excluded.maximum_amount_scale, delay_months = excluded.delay_months",
        )
        .bind(&settings.simulation_id)
        .bind(i64::from(settings.effective_from_month.number()))
        .bind(coefficient)
        .bind(scale)
        .bind(delay)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn load_repair_cost_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RepairCostSettingsDto>, SqliteDataAccessError> {
        let currency = self.simulation_currency(simulation_id).await?;
        let rows = sqlx::query(
            "SELECT * FROM repair_cost_settings WHERE simulation_id = ? ORDER BY effective_from_month",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| mapping::repair_cost_settings(row, currency))
            .collect()
    }
}
