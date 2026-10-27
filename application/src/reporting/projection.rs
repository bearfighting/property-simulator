use property_simulation_account::AccountCatalogue;
use property_simulation_ledger::Ledger;
use property_simulation_money::Currency;
use property_simulation_month::Month;
use property_simulation_reporting::{ReportingInput, audit, balance_sheet, income_statement};

use super::cash_flow::{report as cash_flow_report, summarize as summarize_cash_flow};
use super::result::{
    AccountAmount, FinancialPositionReport, GenerateReportError, PeriodReport, SimulationReport,
};

pub(super) fn build_report(
    simulation_name: String,
    as_of_month: Month,
    accounts: &AccountCatalogue,
    currency: Currency,
    ledger: &Ledger,
) -> Result<SimulationReport, GenerateReportError> {
    let input = ReportingInput::new(accounts, currency, ledger.journal());
    audit(input)?;

    let balance_sheet = balance_sheet(input, as_of_month)?;
    let financial_position = FinancialPositionReport {
        assets: balance_sheet
            .assets()
            .iter()
            .map(|line| AccountAmount {
                account_code: line.account(),
                amount: line.amount(),
            })
            .collect(),
        liabilities: balance_sheet
            .liabilities()
            .iter()
            .map(|line| AccountAmount {
                account_code: line.account(),
                amount: line.amount(),
            })
            .collect(),
        equity_accounts: balance_sheet
            .equity_accounts()
            .iter()
            .map(|line| AccountAmount {
                account_code: line.account(),
                amount: line.amount(),
            })
            .collect(),
        total_assets: balance_sheet.total_assets(),
        total_liabilities: balance_sheet.total_liabilities(),
        equity_account_total: balance_sheet.equity_account_total(),
        cumulative_net_income: balance_sheet.net_income(),
        total_equity: balance_sheet.total_equity(),
    };

    let period = if as_of_month.is_initial() {
        None
    } else {
        Some(build_period_report(accounts, input, as_of_month, currency)?)
    };

    Ok(SimulationReport {
        simulation_name,
        as_of_month,
        completed_months: as_of_month.number(),
        financial_position,
        period,
    })
}

fn build_period_report(
    accounts: &AccountCatalogue,
    input: ReportingInput<'_>,
    end_month: Month,
    currency: Currency,
) -> Result<PeriodReport, GenerateReportError> {
    let start_month = Month::first_calculated();
    let income_statement = income_statement(input, start_month, end_month)?;
    let cash_report = cash_flow_report(accounts, input, start_month, end_month)?;
    let cash_flow = summarize_cash_flow(cash_report.movements(), currency)?;

    Ok(PeriodReport {
        start_month,
        end_month,
        income: income_statement
            .income()
            .iter()
            .map(|line| AccountAmount {
                account_code: line.account(),
                amount: line.amount(),
            })
            .collect(),
        expenses: income_statement
            .expenses()
            .iter()
            .map(|line| AccountAmount {
                account_code: line.account(),
                amount: line.amount(),
            })
            .collect(),
        total_income: income_statement.income_total(),
        total_expenses: income_statement.expense_total(),
        net_income: income_statement.net_income(),
        cash_flow,
    })
}
