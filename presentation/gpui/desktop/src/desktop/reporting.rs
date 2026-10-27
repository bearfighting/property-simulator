//! Bridge the final-report Application work unit to the GPUI presentation contract.

use std::{rc::Rc, sync::Arc};

use property_simulation_account::{AccountCatalogue, AccountRole};
use property_simulation_application::{
    CASH_ROLE, CashFlowCategory, MORTGAGE_ROLE, SimulationReport, generate_report,
};
use property_simulation_data_access_sqlite::SqliteDataAccess;
use property_simulation_gpui_ui::{
    CashFlowViewCategory, ReportAmountView, ReportCashFlowAccountView, ReportCashFlowCategoryView,
    ReportPeriodView, ReportingHandler, ReportingViewData,
};
use property_simulation_lifecycle::WorkUnit;
use tokio::runtime::Runtime;

use super::session::SimulationSession;

pub(super) fn handler(
    runtime: Arc<Runtime>,
    data_access: SqliteDataAccess,
    account_catalogue: AccountCatalogue,
    session: SimulationSession,
) -> ReportingHandler {
    Rc::new(move || {
        let runtime = runtime.clone();
        let data_access = data_access.clone();
        let account_catalogue = account_catalogue.clone();
        let session = session.clone();
        let simulation_id = session.simulation_id();

        Box::pin(async move {
            if session.allowed_work() != WorkUnit::Reporting {
                return Err("reporting is not allowed in the current state".to_owned());
            }
            let simulation_id =
                simulation_id.ok_or_else(|| "simulation identity is missing".to_owned())?;
            let task_accounts = account_catalogue.clone();
            let (sender, receiver) = tokio::sync::oneshot::channel();
            runtime.spawn(async move {
                let report = generate_report(&data_access, &task_accounts, &simulation_id).await;
                let _ = sender.send(report);
            });
            let report = receiver
                .await
                .map_err(|error| format!("report generation task stopped: {error}"))?
                .map_err(|error| error.to_string())?;
            to_view_data(report, &account_catalogue)
        })
    })
}

fn to_view_data(
    report: SimulationReport,
    accounts: &AccountCatalogue,
) -> Result<ReportingViewData, String> {
    let position = report.financial_position;
    let ending_cash = account_balance(CASH_ROLE, &position.assets, accounts)?;
    let mortgage_balance = account_balance(MORTGAGE_ROLE, &position.liabilities, accounts)?;
    let assets = account_lines(&position.assets, accounts)?;
    let liabilities = account_lines(&position.liabilities, accounts)?;
    let equity_accounts = account_lines(&position.equity_accounts, accounts)?;
    let period = report
        .period
        .map(|period| {
            let cash_flow = period.cash_flow;
            Ok::<ReportPeriodView, String>(ReportPeriodView {
                start_month: period.start_month.number(),
                end_month: period.end_month.number(),
                income: account_lines(&period.income, accounts)?,
                expenses: account_lines(&period.expenses, accounts)?,
                total_income: period.total_income.to_string(),
                total_expenses: period.total_expenses.to_string(),
                net_income: period.net_income.to_string(),
                cash_inflows: cash_flow.total_inflows.to_string(),
                cash_outflows: cash_flow.total_outflows.to_string(),
                cash_flow_categories: cash_flow.categories.map(|category| {
                    ReportCashFlowCategoryView {
                        category: view_category(category.category),
                        inflows: category.inflows.to_string(),
                        outflows: category.outflows.to_string(),
                    }
                }),
                cash_flow_accounts: cash_flow
                    .accounts
                    .iter()
                    .map(|account| {
                        Ok(ReportCashFlowAccountView {
                            account: account_name(account.account_code, accounts)?,
                            category: view_category(account.category),
                            inflows: account.inflows.to_string(),
                            outflows: account.outflows.to_string(),
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            })
        })
        .transpose()?;

    Ok(ReportingViewData {
        simulation_name: report.simulation_name,
        as_of_month: report.as_of_month.number(),
        completed_months: report.completed_months,
        ending_cash,
        total_assets: position.total_assets.to_string(),
        mortgage_balance,
        total_liabilities: position.total_liabilities.to_string(),
        total_equity: position.total_equity.to_string(),
        assets,
        liabilities,
        equity_accounts,
        equity_account_total: position.equity_account_total.to_string(),
        cumulative_net_income: position.cumulative_net_income.to_string(),
        period,
    })
}

fn account_lines(
    lines: &[property_simulation_application::AccountAmount],
    accounts: &AccountCatalogue,
) -> Result<Vec<ReportAmountView>, String> {
    lines
        .iter()
        .map(|line| {
            Ok(ReportAmountView {
                label: account_name(line.account_code, accounts)?,
                amount: line.amount.to_string(),
            })
        })
        .collect()
}

fn account_balance(
    role: &str,
    lines: &[property_simulation_application::AccountAmount],
    accounts: &AccountCatalogue,
) -> Result<String, String> {
    let code = role_code(role, accounts)?;
    lines
        .iter()
        .find(|line| line.account_code == code)
        .map(|line| line.amount.to_string())
        .ok_or_else(|| format!("report is missing the balance for account role {role}"))
}

fn role_code(
    role: &str,
    accounts: &AccountCatalogue,
) -> Result<property_simulation_account::AccountCode, String> {
    let role = AccountRole::new(role).map_err(|error| error.to_string())?;
    accounts
        .code_for_role(&role)
        .ok_or_else(|| format!("account role {} is not configured", role.as_str()))
}

fn account_name(
    code: property_simulation_account::AccountCode,
    accounts: &AccountCatalogue,
) -> Result<String, String> {
    accounts
        .get(code)
        .map(|account| account.name().to_owned())
        .ok_or_else(|| format!("account {code} is not configured"))
}

const fn view_category(category: CashFlowCategory) -> CashFlowViewCategory {
    match category {
        CashFlowCategory::Operating => CashFlowViewCategory::Operating,
        CashFlowCategory::Investing => CashFlowViewCategory::Investing,
        CashFlowCategory::Financing => CashFlowViewCategory::Financing,
    }
}
