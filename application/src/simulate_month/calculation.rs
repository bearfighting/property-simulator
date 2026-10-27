use std::collections::BTreeMap;

use property_simulation_account::{AccountCatalogue, AccountCode};
use property_simulation_data_access::dto::{
    FixedExpenseSettingsDto, LoanSettingsDto, RentalSettingsDto, RepairCostInstanceDto,
    RepairCostSettingsDto, RepairOccurrenceSettingsDto, SimulationDto,
};
use property_simulation_money::Money;
use property_simulation_month::Month;
use property_simulation_random_engine::RandomRootKey;
use property_simulation_slice_catalogue::{Event, SliceResult};

use super::{
    MonthSimulationError,
    repair_slices::{RepairSliceInputs, calculate_repair_slices},
    standard_slices::{StandardSliceInputs, calculate_standard_slices},
};

pub(super) struct MonthSliceResults {
    pub events: Vec<Event>,
    pub journal_entries: Vec<property_simulation_journal_entry::JournalEntry>,
    pub new_repair_instances: Vec<RepairCostInstanceDto>,
}

pub(super) struct SliceInputs<'a> {
    pub simulation_id: &'a str,
    pub simulation: &'a SimulationDto,
    pub root_key: &'a RandomRootKey,
    pub accounts: &'a AccountCatalogue,
    pub month: Month,
    pub starting_balances: &'a BTreeMap<AccountCode, Money>,
    pub initial_balances: &'a BTreeMap<AccountCode, Money>,
    pub loan_settings: Option<LoanSettingsDto>,
    pub fixed_expense_settings: Option<FixedExpenseSettingsDto>,
    pub rental_settings: Option<RentalSettingsDto>,
    pub repair_occurrence_settings: Option<RepairOccurrenceSettingsDto>,
    pub repair_cost_settings: Option<RepairCostSettingsDto>,
    pub repair_instances: &'a [RepairCostInstanceDto],
}

pub(super) fn calculate_slices(
    input: SliceInputs<'_>,
) -> Result<MonthSliceResults, MonthSimulationError> {
    let standard = calculate_standard_slices(StandardSliceInputs {
        accounts: input.accounts,
        month: input.month,
        starting_balances: input.starting_balances,
        initial_balances: input.initial_balances,
        loan_settings: input.loan_settings,
        fixed_expense_settings: input.fixed_expense_settings,
        rental_settings: input.rental_settings,
    })?;
    let repairs = calculate_repair_slices(RepairSliceInputs {
        simulation_id: input.simulation_id,
        currency: input.simulation.currency,
        root_key: input.root_key,
        accounts: input.accounts,
        month: input.month,
        occurrence_settings: input.repair_occurrence_settings,
        cost_settings: input.repair_cost_settings,
        instances: input.repair_instances,
    })?;
    let mut results = standard;
    results.extend(repairs.results);

    let mut events = Vec::new();
    let mut journal_entries = Vec::new();
    for result in results {
        collect_result(result, input.month, &mut events, &mut journal_entries)?;
    }
    Ok(MonthSliceResults {
        events,
        journal_entries,
        new_repair_instances: repairs.new_instances,
    })
}

fn collect_result(
    result: SliceResult,
    month: Month,
    events: &mut Vec<Event>,
    journal_entries: &mut Vec<property_simulation_journal_entry::JournalEntry>,
) -> Result<(), MonthSimulationError> {
    if result.month() != month {
        return Err(MonthSimulationError::InvalidData(
            "Slice returned a result for the wrong month".into(),
        ));
    }
    let (_, _, mut result_events, mut result_entries) = result.into_parts();
    events.append(&mut result_events);
    journal_entries.append(&mut result_entries);
    Ok(())
}
