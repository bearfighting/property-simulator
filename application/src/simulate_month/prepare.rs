use property_simulation_data_access::dto::RepairCostInstanceDto;
use property_simulation_journal_entry::JournalEntry;
use property_simulation_month::Month;
use property_simulation_stop_condition::satisfied_conditions;

use super::{
    MonthSimulationError, MonthSimulationOutput,
    calculation::{SliceInputs, calculate_slices},
    ledger::balance,
    load::MonthInputs,
    projection::monthly_cash_flow_changes,
    settings::role_code,
};
use crate::MORTGAGE_ROLE;

pub(super) struct PreparedMonth {
    pub month: Month,
    pub events: Vec<property_simulation_slice_catalogue::Event>,
    pub entries: Vec<JournalEntry>,
    pub new_repair_instances: Vec<RepairCostInstanceDto>,
    pub output: MonthSimulationOutput,
    pub next_journal_sequence: u32,
}

pub(super) fn prepare_month(
    inputs: &MonthInputs<'_>,
) -> Result<PreparedMonth, MonthSimulationError> {
    let calculated = calculate_slices(SliceInputs {
        simulation_id: inputs.simulation_id,
        simulation: &inputs.simulation,
        root_key: &inputs.root_key,
        accounts: inputs.accounts,
        month: inputs.month,
        starting_balances: &inputs.starting_balances,
        initial_balances: &inputs.initial_balances,
        loan_settings: inputs.loan_settings.clone(),
        fixed_expense_settings: inputs.fixed_expense_settings.clone(),
        rental_settings: inputs.rental_settings.clone(),
        repair_occurrence_settings: inputs.repair_occurrence_settings.clone(),
        repair_cost_settings: inputs.repair_cost_settings.clone(),
        repair_instances: &inputs.repair_instances,
    })?;

    let mut candidate_ledger = inputs.ledger.clone();
    for entry in &calculated.journal_entries {
        candidate_ledger
            .append(entry.clone(), inputs.accounts)
            .map_err(|error| MonthSimulationError::calculation("ledger append", error))?;
    }
    let next_balances = candidate_ledger
        .ending_balances(inputs.accounts)
        .map_err(|error| MonthSimulationError::calculation("ending ledger balance", error))?;
    let mortgage = role_code(inputs.accounts, MORTGAGE_ROLE)?;
    let all_loans_paid_off = !balance(&next_balances, mortgage)?.is_negative();
    let satisfied = satisfied_conditions(&inputs.stop_conditions, inputs.month, all_loans_paid_off);
    let cash_flow_changes = monthly_cash_flow_changes(
        inputs.accounts,
        &candidate_ledger,
        inputs.month,
        inputs.simulation.currency,
    )?;
    let output = MonthSimulationOutput {
        month: inputs.month,
        cash_flow_changes,
        satisfied_stop_conditions: satisfied,
    };

    Ok(PreparedMonth {
        month: inputs.month,
        events: calculated.events,
        entries: calculated.journal_entries,
        new_repair_instances: calculated.new_repair_instances,
        output,
        next_journal_sequence: inputs.next_journal_sequence,
    })
}
