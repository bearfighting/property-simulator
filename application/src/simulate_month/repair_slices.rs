use property_simulation_account::{AccountCatalogue, AccountCode};
use property_simulation_data_access::dto::MoneyDto;
use property_simulation_data_access::dto::{
    RepairCostInstanceDto, RepairCostSettingsDto, RepairOccurrenceSettingsDto,
};
use property_simulation_money::Currency;
use property_simulation_month::Month;
use property_simulation_random_engine::RandomRootKey;
use property_simulation_slice::{RepairCostSlice, RepairOccurrenceSlice, StochasticSlice};
use property_simulation_slice_catalogue::{
    Event, RepairCostParameters, RepairCostSliceInput, RepairOccurrenceSliceInput, SliceResult,
};

use super::{
    MonthSimulationError,
    random::random_stream,
    settings::{enabled_repair_occurrence, repair_cost_parameters, required, role_code},
};
use crate::{CASH_ROLE, REPAIR_EXPENSE_ROLE};

pub(super) struct RepairSliceInputs<'a> {
    pub simulation_id: &'a str,
    pub currency: Currency,
    pub root_key: &'a RandomRootKey,
    pub accounts: &'a AccountCatalogue,
    pub month: Month,
    pub occurrence_settings: Option<RepairOccurrenceSettingsDto>,
    pub cost_settings: Option<RepairCostSettingsDto>,
    pub instances: &'a [RepairCostInstanceDto],
}

pub(super) struct RepairSliceResults {
    pub results: Vec<SliceResult>,
    pub new_instances: Vec<RepairCostInstanceDto>,
}

pub(super) fn calculate_repair_slices(
    input: RepairSliceInputs<'_>,
) -> Result<RepairSliceResults, MonthSimulationError> {
    validate_instances(input.simulation_id, input.month, input.instances)?;
    let parameters = repair_cost_parameters(input.cost_settings)?;
    let mut results = Vec::new();
    let mut new_instances = Vec::new();
    if enabled_repair_occurrence(input.occurrence_settings)? {
        calculate_repair_occurrence(
            &mut results,
            &mut new_instances,
            parameters,
            input.simulation_id,
            input.root_key,
            input.month,
            input.instances,
        )?;
    }
    calculate_due_repair_costs(
        &mut results,
        parameters,
        input.simulation_id,
        input.currency,
        input.root_key,
        input.month,
        role_code(input.accounts, CASH_ROLE)?,
        role_code(input.accounts, REPAIR_EXPENSE_ROLE)?,
        input.instances,
    )?;
    Ok(RepairSliceResults {
        results,
        new_instances,
    })
}

fn validate_instances(
    simulation_id: &str,
    month: Month,
    instances: &[RepairCostInstanceDto],
) -> Result<(), MonthSimulationError> {
    for instance in instances {
        instance
            .validate()
            .map_err(|error| MonthSimulationError::InvalidData(error.to_string()))?;
        if instance.simulation_id != simulation_id
            || (instance.settled_month.is_none() && instance.due_month < month)
        {
            return Err(MonthSimulationError::InvalidData(
                "repair cost instance history is inconsistent".into(),
            ));
        }
    }
    Ok(())
}

fn calculate_repair_occurrence(
    results: &mut Vec<SliceResult>,
    new_instances: &mut Vec<RepairCostInstanceDto>,
    parameters: Option<RepairCostParameters>,
    simulation_id: &str,
    root_key: &RandomRootKey,
    month: Month,
    instances: &[RepairCostInstanceDto],
) -> Result<(), MonthSimulationError> {
    let parameters = required(
        parameters,
        "repair occurrence is enabled but repair cost settings are missing",
    )?;
    let last_settlement = instances
        .iter()
        .filter_map(|instance| instance.settled_month)
        .max()
        .unwrap_or(Month::initial());
    let age = u64::from(
        month
            .months_since(last_settlement)
            .map_err(|error| MonthSimulationError::InvalidData(error.to_string()))?,
    );
    let mut random = random_stream(root_key, b"repair-occurrence", month, None);
    let result = RepairOccurrenceSlice::new(RepairOccurrenceSliceInput {
        month,
        months_since_repair_cost: age,
    })
    .simulate(&mut random)
    .map_err(|error| MonthSimulationError::calculation("repair occurrence", error))?;
    if result.events().contains(&Event::RepairOccurred) {
        let delay = u16::try_from(parameters.delay_months().get()).map_err(|_| {
            MonthSimulationError::InvalidData(
                "repair cost delay is outside the supported month range".into(),
            )
        })?;
        let due_month = month
            .checked_add(delay)
            .map_err(|error| MonthSimulationError::InvalidData(error.to_string()))?;
        new_instances.push(RepairCostInstanceDto {
            simulation_id: simulation_id.to_owned(),
            occurrence_month: month,
            due_month,
            maximum_amount: MoneyDto::from_domain(parameters.maximum_amount()),
            settled_month: None,
        });
    }
    results.push(result);
    Ok(())
}

// This is the concrete repair settlement input and its random identity; keeping
// these values explicit makes the per-instance derivation visible at the call site.
#[allow(clippy::too_many_arguments)]
fn calculate_due_repair_costs(
    results: &mut Vec<SliceResult>,
    repair_parameters: Option<RepairCostParameters>,
    simulation_id: &str,
    currency: Currency,
    root_key: &RandomRootKey,
    month: Month,
    cash_account: AccountCode,
    expense_account: AccountCode,
    instances: &[RepairCostInstanceDto],
) -> Result<(), MonthSimulationError> {
    for instance in instances
        .iter()
        .filter(|item| item.due_month == month && item.settled_month.is_none())
    {
        if instance.simulation_id != simulation_id {
            return Err(MonthSimulationError::InvalidData(
                "repair cost instance belongs to another simulation".into(),
            ));
        }
        let maximum = instance
            .maximum_amount
            .to_domain(currency)
            .map_err(|error| MonthSimulationError::InvalidData(error.to_string()))?;
        let delay = repair_parameters
            .as_ref()
            .ok_or_else(|| {
                MonthSimulationError::InvalidData(
                    "pending repair cost has no repair cost settings".into(),
                )
            })?
            .delay_months();
        let parameters = RepairCostParameters::new(maximum, delay)
            .map_err(|error| MonthSimulationError::InvalidData(error.to_string()))?;
        let input = RepairCostSliceInput {
            month,
            occurrence_month: instance.occurrence_month,
            cash_account,
            expense_account,
        };
        let mut random = random_stream(
            root_key,
            b"repair-cost",
            month,
            Some(instance.occurrence_month),
        );
        results.push(
            RepairCostSlice::new(parameters, input)
                .simulate(&mut random)
                .map_err(|error| MonthSimulationError::calculation("repair cost", error))?,
        );
    }
    Ok(())
}
