use property_simulation_data_access::{
    DataAccess,
    dto::{
        FixedExpenseSettingsDto, LoanSettingsDto, RentalSettingsDto, RepairCostSettingsDto,
        RepairOccurrenceSettingsDto,
    },
};

use super::types::{InitialSliceSettings, InitializeSimulationError};

pub(super) async fn save<D: DataAccess>(
    data_access: &D,
    simulation_id: &str,
    settings: InitialSliceSettings,
) -> Result<(), InitializeSimulationError> {
    if let Some(configured) = settings.loan {
        data_access
            .save_loan_settings(&LoanSettingsDto {
                simulation_id: simulation_id.to_owned(),
                enabled: Some(configured.enabled),
                parameters: Some(configured.parameters),
            })
            .await?;
    }
    if let Some(configured) = settings.fixed_expense {
        data_access
            .save_fixed_expense_settings(&FixedExpenseSettingsDto {
                simulation_id: simulation_id.to_owned(),
                enabled: Some(configured.enabled),
                parameters: Some(configured.parameters),
            })
            .await?;
    }
    if let Some(configured) = settings.rental {
        data_access
            .save_rental_settings(&RentalSettingsDto {
                simulation_id: simulation_id.to_owned(),
                enabled: Some(configured.enabled),
                parameters: Some(configured.parameters),
            })
            .await?;
    }
    if let Some(enabled) = settings.repair_occurrence_enabled {
        data_access
            .save_repair_occurrence_settings(&RepairOccurrenceSettingsDto {
                simulation_id: simulation_id.to_owned(),
                enabled: Some(enabled),
            })
            .await?;
    }
    if let Some(parameters) = settings.repair_cost {
        data_access
            .save_repair_cost_settings(&RepairCostSettingsDto {
                simulation_id: simulation_id.to_owned(),
                parameters: Some(parameters),
            })
            .await?;
    }
    Ok(())
}
