use property_simulation_account::{AccountCatalogue, AccountCode, AccountRole};
use property_simulation_data_access::dto::{
    FixedExpenseSettingsDto, LoanSettingsDto, RentalSettingsDto, RepairCostSettingsDto,
    RepairOccurrenceSettingsDto,
};
use property_simulation_slice_catalogue::{
    FixedExpenseParameters, LoanParameters, RentalParameters, RepairCostParameters,
};

use crate::ConfiguredSlice;

use super::MonthSimulationError;

pub(super) fn role_code(
    accounts: &AccountCatalogue,
    role: &str,
) -> Result<AccountCode, MonthSimulationError> {
    let role_value = AccountRole::new(role).expect("static account roles are valid");
    accounts.code_for_role(&role_value).ok_or_else(|| {
        MonthSimulationError::InvalidData(format!("account role {role} is not configured"))
    })
}

pub(super) fn enabled_loan(
    settings: Option<LoanSettingsDto>,
) -> Result<Option<ConfiguredSlice<LoanParameters>>, MonthSimulationError> {
    let Some(settings) = settings else {
        return Ok(None);
    };
    let enabled = settings.enabled.ok_or_else(|| {
        MonthSimulationError::InvalidData("loan setting has no enabled value".into())
    })?;
    if !enabled {
        return Ok(None);
    }
    Ok(Some(ConfiguredSlice {
        enabled: true,
        parameters: required(
            settings.parameters,
            "enabled loan settings are missing parameters",
        )?,
    }))
}

pub(super) fn enabled_fixed_expense(
    settings: Option<FixedExpenseSettingsDto>,
) -> Result<Option<ConfiguredSlice<FixedExpenseParameters>>, MonthSimulationError> {
    let Some(settings) = settings else {
        return Ok(None);
    };
    let enabled = settings.enabled.ok_or_else(|| {
        MonthSimulationError::InvalidData("fixed expense setting has no enabled value".into())
    })?;
    if !enabled {
        return Ok(None);
    }
    Ok(Some(ConfiguredSlice {
        enabled: true,
        parameters: required(
            settings.parameters,
            "enabled fixed expense settings are missing parameters",
        )?,
    }))
}

pub(super) fn enabled_rental(
    settings: Option<RentalSettingsDto>,
) -> Result<Option<ConfiguredSlice<RentalParameters>>, MonthSimulationError> {
    let Some(settings) = settings else {
        return Ok(None);
    };
    let enabled = settings.enabled.ok_or_else(|| {
        MonthSimulationError::InvalidData("rental setting has no enabled value".into())
    })?;
    if !enabled {
        return Ok(None);
    }
    Ok(Some(ConfiguredSlice {
        enabled: true,
        parameters: required(
            settings.parameters,
            "enabled rental settings are missing parameters",
        )?,
    }))
}

pub(super) fn enabled_repair_occurrence(
    settings: Option<RepairOccurrenceSettingsDto>,
) -> Result<bool, MonthSimulationError> {
    let Some(settings) = settings else {
        return Ok(false);
    };
    settings.enabled.ok_or_else(|| {
        MonthSimulationError::InvalidData("repair occurrence setting has no enabled value".into())
    })
}

pub(super) fn repair_cost_parameters(
    settings: Option<RepairCostSettingsDto>,
) -> Result<Option<RepairCostParameters>, MonthSimulationError> {
    settings
        .and_then(|settings| settings.parameters)
        .map(|parameters| {
            RepairCostParameters::new(parameters.maximum_amount(), parameters.delay_months())
                .map_err(|error| MonthSimulationError::InvalidData(error.to_string()))
        })
        .transpose()
}

pub(super) fn required<T>(value: Option<T>, message: &str) -> Result<T, MonthSimulationError> {
    value.ok_or_else(|| MonthSimulationError::InvalidData(message.to_owned()))
}
