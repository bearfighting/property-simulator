use property_simulation_account::AccountCatalogue;
use property_simulation_data_access::dto::{
    JournalEntryDto, SimulationAccountDto, SimulationMonthDto,
};
use property_simulation_ledger::Ledger;
use property_simulation_month::Month;

use super::GenerateReportError;

pub(super) fn validate_months(
    simulation_id: &str,
    months: &[SimulationMonthDto],
) -> Result<Month, GenerateReportError> {
    if months.is_empty() {
        return Err(GenerateReportError::InvalidData(
            "simulation has no initialized Month 0 record".into(),
        ));
    }

    let mut expected = Month::initial();
    for (index, saved) in months.iter().enumerate() {
        if saved.simulation_id != simulation_id || saved.month != expected {
            return Err(GenerateReportError::InvalidData(format!(
                "simulation month history is incomplete or out of order at month {expected}"
            )));
        }
        if index + 1 < months.len() {
            expected = expected.next().map_err(|error| {
                GenerateReportError::InvalidData(format!("simulation month overflow: {error}"))
            })?;
        }
    }

    months
        .last()
        .map(|saved| saved.month)
        .ok_or_else(|| GenerateReportError::InvalidData("simulation month history is empty".into()))
}

pub(super) fn validate_account_membership(
    simulation_id: &str,
    catalogue: &AccountCatalogue,
    persisted: &[SimulationAccountDto],
) -> Result<(), GenerateReportError> {
    let mut seen = std::collections::BTreeSet::new();
    if persisted.len() != catalogue.len()
        || persisted.iter().any(|account| {
            account.simulation_id != simulation_id
                || catalogue.get(account.account_code).is_none()
                || !seen.insert(account.account_code)
        })
    {
        return Err(GenerateReportError::InvalidData(
            "desktop account catalogue does not match the initialized simulation".into(),
        ));
    }
    Ok(())
}

pub(super) fn rebuild(
    simulation_id: &str,
    currency: property_simulation_money::Currency,
    accounts: &AccountCatalogue,
    as_of_month: Month,
    mut entries: Vec<JournalEntryDto>,
) -> Result<Ledger, GenerateReportError> {
    entries.sort_by_key(|entry| (entry.month, entry.sequence));
    let mut ledger = Ledger::new(currency);
    for entry in entries {
        if entry.simulation_id != simulation_id {
            return Err(GenerateReportError::InvalidData(
                "journal entry belongs to another simulation".into(),
            ));
        }
        if entry.month > as_of_month {
            return Err(GenerateReportError::InvalidData(format!(
                "journal entry in month {} is later than the last saved month {as_of_month}",
                entry.month
            )));
        }
        let entry = entry
            .domain_entry(currency)
            .map_err(|error| GenerateReportError::InvalidData(error.to_string()))?;
        ledger
            .append(entry, accounts)
            .map_err(|error| GenerateReportError::LedgerCalculation {
                operation: "Ledger reconstruction",
                message: error.to_string(),
            })?;
    }
    Ok(ledger)
}
