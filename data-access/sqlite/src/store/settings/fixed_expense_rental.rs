use crate::{SqliteDataAccessError, mapping, store::SqliteDataAccess};
use property_simulation_data_access::dto::{FixedExpenseSettingsDto, RentalSettingsDto};
use property_simulation_slice_catalogue::FixedExpenseMethod;

use super::common::{bind_money, optional_payload, validate_currency, validate_effective_month};

impl SqliteDataAccess {
    pub async fn save_fixed_expense_settings(
        &self,
        settings: &FixedExpenseSettingsDto,
    ) -> Result<(), SqliteDataAccessError> {
        validate_effective_month(settings.effective_from_month)?;
        optional_payload(
            settings.enabled,
            settings.parameters.is_some(),
            "fixed expense settings",
        )?;
        let currency = self.simulation_currency(&settings.simulation_id).await?;
        let (method, coefficient, scale, annual_rate) = match settings.parameters.as_ref() {
            None => (None, None, None, None),
            Some(parameters) => match parameters.method() {
                FixedExpenseMethod::MonthlyAmount(amount) => {
                    validate_currency(*amount, currency)?;
                    let (coefficient, scale) = bind_money(*amount);
                    (Some("monthly_amount"), Some(coefficient), Some(scale), None)
                }
                FixedExpenseMethod::AnnualPropertyValueRate(rate) => (
                    Some("annual_property_value_rate"),
                    None,
                    None,
                    Some(rate.to_string()),
                ),
            },
        };
        sqlx::query(
            "INSERT INTO fixed_expense_settings \
             (simulation_id, effective_from_month, enabled, method, monthly_amount_coefficient, monthly_amount_scale, annual_property_value_rate) \
             VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(simulation_id, effective_from_month) DO UPDATE SET \
             enabled = excluded.enabled, method = excluded.method, \
             monthly_amount_coefficient = excluded.monthly_amount_coefficient, \
             monthly_amount_scale = excluded.monthly_amount_scale, \
             annual_property_value_rate = excluded.annual_property_value_rate",
        )
        .bind(&settings.simulation_id)
        .bind(i64::from(settings.effective_from_month.number()))
        .bind(settings.enabled.map(i64::from))
        .bind(method)
        .bind(coefficient)
        .bind(scale)
        .bind(annual_rate)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_fixed_expense_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<FixedExpenseSettingsDto>, SqliteDataAccessError> {
        let currency = self.simulation_currency(simulation_id).await?;
        let rows = sqlx::query(
            "SELECT * FROM fixed_expense_settings WHERE simulation_id = ? ORDER BY effective_from_month",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| mapping::fixed_expense_settings(row, currency))
            .collect()
    }

    pub async fn save_rental_settings(
        &self,
        settings: &RentalSettingsDto,
    ) -> Result<(), SqliteDataAccessError> {
        validate_effective_month(settings.effective_from_month)?;
        optional_payload(
            settings.enabled,
            settings.parameters.is_some(),
            "rental settings",
        )?;
        let currency = self.simulation_currency(&settings.simulation_id).await?;
        if let Some(parameters) = settings.parameters.as_ref() {
            validate_currency(parameters.monthly_rent(), currency)?;
        }
        let (coefficient, scale) =
            settings
                .parameters
                .as_ref()
                .map_or((None, None), |parameters| {
                    let (coefficient, scale) = bind_money(parameters.monthly_rent());
                    (Some(coefficient), Some(scale))
                });
        sqlx::query(
            "INSERT INTO rental_settings \
             (simulation_id, effective_from_month, enabled, monthly_rent_coefficient, monthly_rent_scale) \
             VALUES (?, ?, ?, ?, ?) ON CONFLICT(simulation_id, effective_from_month) DO UPDATE SET \
             enabled = excluded.enabled, monthly_rent_coefficient = excluded.monthly_rent_coefficient, \
             monthly_rent_scale = excluded.monthly_rent_scale",
        )
        .bind(&settings.simulation_id)
        .bind(i64::from(settings.effective_from_month.number()))
        .bind(settings.enabled.map(i64::from))
        .bind(coefficient)
        .bind(scale)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_rental_settings(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RentalSettingsDto>, SqliteDataAccessError> {
        let currency = self.simulation_currency(simulation_id).await?;
        let rows = sqlx::query(
            "SELECT * FROM rental_settings WHERE simulation_id = ? ORDER BY effective_from_month",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| mapping::rental_settings(row, currency))
            .collect()
    }
}
