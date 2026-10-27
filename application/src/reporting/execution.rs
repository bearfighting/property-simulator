use property_simulation_account::AccountCatalogue;
use property_simulation_data_access::DataAccess;

use super::{GenerateReportError, SimulationReport, ledger, projection};

/// Load persisted facts and derive the final report for one simulation.
///
/// A Month 0-only run returns its opening financial position and no period
/// report. Reporting calculations never change lifecycle state or persistence.
pub async fn generate_report<D: DataAccess>(
    data_access: &D,
    account_catalogue: &AccountCatalogue,
    simulation_id: &str,
) -> Result<SimulationReport, GenerateReportError> {
    let simulation = data_access
        .load_simulation(simulation_id)
        .await?
        .ok_or_else(|| GenerateReportError::SimulationNotFound(simulation_id.to_owned()))?;
    if simulation.id != simulation_id {
        return Err(GenerateReportError::InvalidData(
            "loaded simulation identity does not match the requested simulation".into(),
        ));
    }

    let months = data_access.load_months(simulation_id).await?;
    let as_of_month = ledger::validate_months(simulation_id, &months)?;
    let persisted_accounts = data_access.load_accounts(simulation_id).await?;
    ledger::validate_account_membership(simulation_id, account_catalogue, &persisted_accounts)?;

    let entries = data_access.load_journal_entries(simulation_id).await?;
    let ledger = ledger::rebuild(
        simulation_id,
        simulation.currency,
        account_catalogue,
        as_of_month,
        entries,
    )?;

    projection::build_report(
        simulation.name,
        as_of_month,
        account_catalogue,
        simulation.currency,
        &ledger,
    )
}
