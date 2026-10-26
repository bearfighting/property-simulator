use crate::{SqliteDataAccessError, mapping, store::SqliteDataAccess};
use property_simulation_data_access::dto::{RepairCostSettingsDto, RepairOccurrenceSettingsDto};

use super::common::{bind_money, validate_currency};

impl SqliteDataAccess {
    pub(crate) async fn save_repair_occurrence_settings(
        &self,
        settings: &RepairOccurrenceSettingsDto,
    ) -> Result<(), SqliteDataAccessError> {
        sqlx::query(
            "INSERT INTO repair_occurrence_settings (simulation_id, enabled) VALUES (?, ?)",
        )
        .bind(&settings.simulation_id)
        .bind(settings.enabled.map(i64::from))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn load_repair_occurrence_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Option<RepairOccurrenceSettingsDto>, SqliteDataAccessError> {
        let row = sqlx::query("SELECT * FROM repair_occurrence_settings WHERE simulation_id = ?")
            .bind(simulation_id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref()
            .map(mapping::repair_occurrence_settings)
            .transpose()
    }

    pub(crate) async fn save_repair_cost_settings(
        &self,
        settings: &RepairCostSettingsDto,
    ) -> Result<(), SqliteDataAccessError> {
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
             (simulation_id, maximum_amount_coefficient, maximum_amount_scale, delay_months) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&settings.simulation_id)
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
    ) -> Result<Option<RepairCostSettingsDto>, SqliteDataAccessError> {
        let currency = self.simulation_currency(simulation_id).await?;
        let row = sqlx::query("SELECT * FROM repair_cost_settings WHERE simulation_id = ?")
            .bind(simulation_id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref()
            .map(|row| mapping::repair_cost_settings(row, currency))
            .transpose()
    }
}
