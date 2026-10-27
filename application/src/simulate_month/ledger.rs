use std::collections::{BTreeMap, BTreeSet};

use property_simulation_account::{AccountCatalogue, AccountCode};
use property_simulation_data_access::dto::{
    JournalEntryDto, SimulationAccountDto, SimulationDto, SimulationMonthDto,
};
use property_simulation_ledger::Ledger;
use property_simulation_money::Money;
use property_simulation_month::Month;

use super::MonthSimulationError;

pub(super) fn build_ledger(
    simulation: &SimulationDto,
    accounts: &AccountCatalogue,
    mut entries: Vec<JournalEntryDto>,
) -> Result<Ledger, MonthSimulationError> {
    entries.sort_by_key(|entry| (entry.month, entry.sequence));
    let mut ledger = Ledger::new(simulation.currency);
    for entry in entries {
        if entry.simulation_id != simulation.id {
            return Err(MonthSimulationError::InvalidData(
                "journal entry belongs to another simulation".into(),
            ));
        }
        let domain_entry = entry
            .domain_entry(simulation.currency)
            .map_err(|error| MonthSimulationError::InvalidData(error.to_string()))?;
        ledger
            .append(domain_entry, accounts)
            .map_err(|error| MonthSimulationError::calculation("ledger reconstruction", error))?;
    }
    Ok(ledger)
}

pub(super) fn next_month(
    simulation_id: &str,
    months: &[SimulationMonthDto],
) -> Result<Month, MonthSimulationError> {
    let mut expected = Month::initial();
    for saved in months {
        if saved.simulation_id != simulation_id || saved.month != expected {
            return Err(MonthSimulationError::InvalidData(format!(
                "simulation month history is incomplete at month {expected}"
            )));
        }
        expected = expected
            .next()
            .map_err(|error| MonthSimulationError::InvalidData(error.to_string()))?;
    }
    if months.is_empty() {
        return Err(MonthSimulationError::InvalidData(
            "simulation has not been initialized at Month 0".into(),
        ));
    }
    Ok(expected)
}

pub(super) fn validate_account_membership(
    simulation_id: &str,
    accounts: &AccountCatalogue,
    persisted: &[SimulationAccountDto],
) -> Result<(), MonthSimulationError> {
    let mut seen = BTreeSet::new();
    if persisted.len() != accounts.len()
        || persisted.iter().any(|account| {
            account.simulation_id != simulation_id
                || accounts.get(account.account_code).is_none()
                || !seen.insert(account.account_code)
        })
    {
        return Err(MonthSimulationError::InvalidData(
            "desktop account catalogue does not match the initialized simulation".into(),
        ));
    }
    Ok(())
}

pub(super) fn balance(
    balances: &BTreeMap<AccountCode, Money>,
    account: AccountCode,
) -> Result<Money, MonthSimulationError> {
    balances.get(&account).copied().ok_or_else(|| {
        MonthSimulationError::InvalidData(format!("account {account} has no Ledger balance"))
    })
}
