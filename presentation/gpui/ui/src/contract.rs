use gpui_kit::App;
use property_simulation_lifecycle::SimulationState;
use std::{future::Future, pin::Pin, rc::Rc};

/// UI action passed from the application shell into a page.
pub(crate) type PageAction = Rc<dyn Fn(&mut App)>;

/// Values collected by the simulation initialization page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializationFormData {
    pub name: String,
    pub property_value: String,
    pub cash_balance: String,
    pub stop_after_months: String,
    pub stop_when_all_loans_paid_off: bool,
    pub loan_enabled: bool,
    pub loan_balance: String,
    pub annual_interest_rate_percent: String,
    pub loan_term_months: String,
    pub rental_enabled: bool,
    pub monthly_rent: String,
    pub fixed_expense_enabled: bool,
    pub monthly_expenses: String,
    pub repairs_enabled: bool,
    pub repair_maximum_amount: String,
    pub repair_delay_months: String,
}

/// Lifecycle state returned by the host after successful initialization.
pub type SubmissionResult = Result<SimulationState, String>;

/// Async host callback. The future is local to GPUI's foreground executor.
pub type InitializationHandler =
    Rc<dyn Fn(InitializationFormData) -> Pin<Box<dyn Future<Output = SubmissionResult> + 'static>>>;

/// Categories rendered by the monthly cash-flow charts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CashFlowViewCategory {
    Operating,
    Investing,
    Financing,
}

/// One category's display amount and normalized pie-chart share.
#[derive(Debug, Clone, PartialEq)]
pub struct CashFlowChartSlice {
    pub label: String,
    pub category: CashFlowViewCategory,
    pub amount: String,
    pub share: f32,
}

/// Presentation-ready output for one completed month.
#[derive(Debug, Clone, PartialEq)]
pub struct MonthlySimulationViewData {
    pub month_number: u16,
    pub inflow_total: String,
    pub outflow_total: String,
    pub inflows: Vec<CashFlowChartSlice>,
    pub outflows: Vec<CashFlowChartSlice>,
    pub stop_reason: Option<String>,
}

/// Result of one monthly operation, including the host's updated lifecycle state.
#[derive(Debug, Clone, PartialEq)]
pub struct MonthlySimulationResponse {
    pub state: SimulationState,
    pub result: Result<MonthlySimulationViewData, String>,
}

/// Async host callback for simulating and persisting one complete month.
pub type MonthlySimulationHandler =
    Rc<dyn Fn() -> Pin<Box<dyn Future<Output = MonthlySimulationResponse> + 'static>>>;

/// Display-ready amount for one account or summary metric.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportAmountView {
    pub label: String,
    pub amount: String,
}

/// One cash-flow category's cumulative signed movements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportCashFlowCategoryView {
    pub category: CashFlowViewCategory,
    pub inflows: String,
    pub outflows: String,
}

/// Cumulative cash movements attributed to one account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportCashFlowAccountView {
    pub account: String,
    pub category: CashFlowViewCategory,
    pub inflows: String,
    pub outflows: String,
}

/// Period income, expenses, and cash-flow totals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPeriodView {
    pub start_month: u16,
    pub end_month: u16,
    pub income: Vec<ReportAmountView>,
    pub expenses: Vec<ReportAmountView>,
    pub total_income: String,
    pub total_expenses: String,
    pub net_income: String,
    pub cash_inflows: String,
    pub cash_outflows: String,
    pub cash_flow_categories: [ReportCashFlowCategoryView; 3],
    pub cash_flow_accounts: Vec<ReportCashFlowAccountView>,
}

/// Values needed to render the simulation-wide final report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportingViewData {
    pub simulation_name: String,
    pub as_of_month: u16,
    pub completed_months: u16,
    pub ending_cash: String,
    pub total_assets: String,
    pub mortgage_balance: String,
    pub total_liabilities: String,
    pub total_equity: String,
    pub assets: Vec<ReportAmountView>,
    pub liabilities: Vec<ReportAmountView>,
    pub equity_accounts: Vec<ReportAmountView>,
    pub equity_account_total: String,
    pub cumulative_net_income: String,
    pub period: Option<ReportPeriodView>,
}

/// Async host callback for reading and deriving the final report.
pub type ReportingHandler =
    Rc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<ReportingViewData, String>> + 'static>>>;

/// Desktop callback for a user-requested end of the active simulation run.
/// The host returns the updated lifecycle state, or `None` if the run is not
/// currently in a state where it can be ended.
pub type StopSimulationHandler = Rc<dyn Fn() -> Option<SimulationState>>;

/// Desktop callback for abandoning the active run and starting a fresh one.
pub type ResetSimulationHandler = Rc<dyn Fn() -> Option<SimulationState>>;
