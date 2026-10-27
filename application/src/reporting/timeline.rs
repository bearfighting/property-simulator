//! Load persisted monthly cash-flow projections for timeline browsing.

use property_simulation_account::AccountCatalogue;
use property_simulation_data_access::DataAccess;
use property_simulation_month::Month;
use property_simulation_reporting::ReportingInput;

use super::{CashFlowSummary, GenerateReportError, cash_flow, ledger};

/// One completed month and its Ledger-derived cash-flow projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthTimelineItem {
    pub month: Month,
    pub cash_flow: CashFlowSummary,
}

/// Load each completed month without rerunning its simulation calculation.
pub async fn load_month_timeline<D: DataAccess>(
    data_access: &D,
    account_catalogue: &AccountCatalogue,
    simulation_id: &str,
) -> Result<Vec<MonthTimelineItem>, GenerateReportError> {
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
    let input = ReportingInput::new(account_catalogue, simulation.currency, ledger.journal());

    if as_of_month.is_initial() {
        return Ok(Vec::new());
    }

    cash_flow::reports_by_month(
        account_catalogue,
        input,
        Month::first_calculated(),
        as_of_month,
    )?
    .into_iter()
    .map(|report| {
        let month = report.end();
        let cash_flow = cash_flow::summarize(report.movements(), simulation.currency)?;
        Ok(MonthTimelineItem { month, cash_flow })
    })
    .collect()
}
