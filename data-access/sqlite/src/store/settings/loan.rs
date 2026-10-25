use crate::{SqliteDataAccessError, mapping, store::SqliteDataAccess};
use property_simulation_data_access::dto::LoanSettingsDto;

use super::common::{optional_payload, validate_effective_month};

impl SqliteDataAccess {
    pub async fn save_loan_settings(
        &self,
        settings: &LoanSettingsDto,
    ) -> Result<(), SqliteDataAccessError> {
        validate_effective_month(settings.effective_from_month)?;
        optional_payload(
            settings.enabled,
            settings.parameters.is_some(),
            "loan settings",
        )?;
        let (rate, term) = settings
            .parameters
            .as_ref()
            .map_or((None, None), |parameters| {
                (
                    Some(parameters.annual_interest_rate().to_string()),
                    Some(i64::from(parameters.remaining_term_months_at_start().get())),
                )
            });
        sqlx::query(
            "INSERT INTO loan_settings \
             (simulation_id, effective_from_month, enabled, annual_interest_rate, remaining_term_months_at_start) \
             VALUES (?, ?, ?, ?, ?) ON CONFLICT(simulation_id, effective_from_month) DO UPDATE SET \
             enabled = excluded.enabled, annual_interest_rate = excluded.annual_interest_rate, \
             remaining_term_months_at_start = excluded.remaining_term_months_at_start",
        )
        .bind(&settings.simulation_id)
        .bind(i64::from(settings.effective_from_month.number()))
        .bind(settings.enabled.map(i64::from))
        .bind(rate)
        .bind(term)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_loan_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<LoanSettingsDto>, SqliteDataAccessError> {
        let rows = sqlx::query(
            "SELECT * FROM loan_settings WHERE simulation_id = ? ORDER BY effective_from_month",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(mapping::loan_settings).collect()
    }
}
