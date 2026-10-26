use crate::{SqliteDataAccessError, mapping, store::SqliteDataAccess};
use property_simulation_data_access::dto::LoanSettingsDto;

use super::common::optional_payload;

impl SqliteDataAccess {
    pub(crate) async fn save_loan_settings(
        &self,
        settings: &LoanSettingsDto,
    ) -> Result<(), SqliteDataAccessError> {
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
             (simulation_id, enabled, annual_interest_rate, remaining_term_months_at_start) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&settings.simulation_id)
        .bind(settings.enabled.map(i64::from))
        .bind(rate)
        .bind(term)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn load_loan_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Option<LoanSettingsDto>, SqliteDataAccessError> {
        let row = sqlx::query("SELECT * FROM loan_settings WHERE simulation_id = ?")
            .bind(simulation_id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(mapping::loan_settings).transpose()
    }
}
