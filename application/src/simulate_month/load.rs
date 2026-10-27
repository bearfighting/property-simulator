use property_simulation_account::AccountCatalogue;
use property_simulation_data_access::{
    DataAccess,
    dto::{
        FixedExpenseSettingsDto, JournalEntryDto, LoanSettingsDto, RentalSettingsDto,
        RepairCostInstanceDto, RepairCostSettingsDto, RepairOccurrenceSettingsDto, SimulationDto,
    },
};
use property_simulation_ledger::Ledger;
use property_simulation_money::Money;
use property_simulation_month::Month;
use property_simulation_random_engine::RandomRootKey;
use property_simulation_stop_condition::StopCondition;
use std::collections::BTreeMap;

use super::{
    MonthSimulationError,
    ledger::{build_ledger, next_month, validate_account_membership},
    settings::required,
};

/// Persisted and derived facts needed for one invocation; never retained between calls.
pub(super) struct MonthInputs<'a> {
    pub simulation_id: &'a str,
    pub simulation: SimulationDto,
    pub month: Month,
    pub root_key: RandomRootKey,
    pub accounts: &'a AccountCatalogue,
    pub ledger: Ledger,
    pub starting_balances: BTreeMap<property_simulation_account::AccountCode, Money>,
    pub initial_balances: BTreeMap<property_simulation_account::AccountCode, Money>,
    pub next_journal_sequence: u32,
    pub loan_settings: Option<LoanSettingsDto>,
    pub fixed_expense_settings: Option<FixedExpenseSettingsDto>,
    pub rental_settings: Option<RentalSettingsDto>,
    pub repair_occurrence_settings: Option<RepairOccurrenceSettingsDto>,
    pub repair_cost_settings: Option<RepairCostSettingsDto>,
    pub stop_conditions: Vec<StopCondition>,
    pub repair_instances: Vec<RepairCostInstanceDto>,
}

pub(super) async fn load_month_inputs<'a, D: DataAccess>(
    data_access: &D,
    accounts: &'a AccountCatalogue,
    simulation_id: &'a str,
) -> Result<MonthInputs<'a>, MonthSimulationError> {
    let simulation = required(
        data_access.load_simulation(simulation_id).await?,
        "simulation does not exist",
    )?;
    let root_key_dto = required(
        data_access.load_root_key(simulation_id).await?,
        "simulation random root is missing",
    )?;
    if root_key_dto.simulation_id != simulation_id {
        return Err(MonthSimulationError::InvalidData(
            "simulation random root belongs to another simulation".into(),
        ));
    }

    let month = next_month(
        simulation_id,
        &data_access.load_months(simulation_id).await?,
    )?;
    validate_account_membership(
        simulation_id,
        accounts,
        &data_access.load_accounts(simulation_id).await?,
    )?;

    let journal_entries = data_access.load_journal_entries(simulation_id).await?;
    let next_journal_sequence = next_sequence(&journal_entries)?;
    let initial_entries = journal_entries
        .iter()
        .filter(|entry| entry.month.is_initial())
        .cloned()
        .collect();
    let ledger = build_ledger(&simulation, accounts, journal_entries)?;
    let starting_balances = ledger
        .ending_balances(accounts)
        .map_err(|error| MonthSimulationError::calculation("ledger balance", error))?;
    let initial_ledger = build_ledger(&simulation, accounts, initial_entries)?;
    let initial_balances = initial_ledger
        .ending_balances(accounts)
        .map_err(|error| MonthSimulationError::calculation("opening ledger balance", error))?;

    let stop_conditions = data_access.load_stop_conditions(simulation_id).await?;
    if stop_conditions.simulation_id != simulation_id {
        return Err(MonthSimulationError::InvalidData(
            "stop conditions belong to another simulation".into(),
        ));
    }

    Ok(MonthInputs {
        simulation_id,
        simulation,
        month,
        root_key: root_key_dto.domain_key(),
        accounts,
        ledger,
        starting_balances,
        initial_balances,
        next_journal_sequence,
        loan_settings: data_access.load_loan_settings(simulation_id).await?,
        fixed_expense_settings: data_access
            .load_fixed_expense_settings(simulation_id)
            .await?,
        rental_settings: data_access.load_rental_settings(simulation_id).await?,
        repair_occurrence_settings: data_access
            .load_repair_occurrence_settings(simulation_id)
            .await?,
        repair_cost_settings: data_access.load_repair_cost_settings(simulation_id).await?,
        stop_conditions: stop_conditions.conditions,
        repair_instances: data_access
            .load_repair_cost_instances(simulation_id)
            .await?,
    })
}

fn next_sequence(entries: &[JournalEntryDto]) -> Result<u32, MonthSimulationError> {
    entries
        .iter()
        .map(|entry| entry.sequence)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| MonthSimulationError::InvalidData("journal sequence overflow".into()))
}
