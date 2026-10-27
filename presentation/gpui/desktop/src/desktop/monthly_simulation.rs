//! Bridge the monthly Application work unit to the GPUI presentation contract.

use std::{rc::Rc, sync::Arc};

use property_simulation_account::AccountCatalogue;
use property_simulation_application::{CashFlowCategory, MonthSimulationOutput, simulate_month};
use property_simulation_data_access_sqlite::SqliteDataAccess;
use property_simulation_gpui_ui::{
    CashFlowChartSlice, CashFlowViewCategory, MonthlySimulationHandler, MonthlySimulationResponse,
    MonthlySimulationViewData,
};
use property_simulation_lifecycle::WorkUnit;
use property_simulation_money::Money;
use property_simulation_stop_condition::StopCondition;
use tokio::runtime::Runtime;

use super::session::SimulationSession;

pub(super) fn handler(
    runtime: Arc<Runtime>,
    data_access: SqliteDataAccess,
    account_catalogue: AccountCatalogue,
    session: SimulationSession,
) -> MonthlySimulationHandler {
    Rc::new(move || {
        let runtime = runtime.clone();
        let data_access = data_access.clone();
        let account_catalogue = account_catalogue.clone();
        let session = session.clone();
        let simulation_id = session.simulation_id();

        Box::pin(async move {
            let current = session.state();
            if current.allowed_work() != WorkUnit::MonthSimulation {
                return MonthlySimulationResponse {
                    state: current,
                    result: Err("monthly simulation is not allowed in the current state".into()),
                };
            }

            let result = if let Some(simulation_id) = simulation_id {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                runtime.spawn(async move {
                    let result =
                        simulate_month(&data_access, &account_catalogue, &simulation_id).await;
                    let _ = sender.send(result);
                });
                receiver
                    .await
                    .map_err(|error| format!("monthly simulation task stopped: {error}"))
                    .and_then(|result| result.map_err(|error| error.to_string()))
            } else {
                Err("simulation identity is missing".to_owned())
            };

            let should_stop = result
                .as_ref()
                .map(|output| !output.satisfied_stop_conditions.is_empty())
                .unwrap_or(true);
            let state = session.finish_month(should_stop);
            let result = result.map(to_view_data);
            MonthlySimulationResponse { state, result }
        })
    })
}

fn to_view_data(output: MonthSimulationOutput) -> MonthlySimulationViewData {
    let changes = output.cash_flow_changes;
    let inflows = account_slices(&changes.accounts, true);
    let outflows = account_slices(&changes.accounts, false);
    let stop_reason = if output.satisfied_stop_conditions.is_empty() {
        None
    } else {
        Some(
            output
                .satisfied_stop_conditions
                .iter()
                .map(stop_condition_label)
                .collect::<Vec<_>>()
                .join(" · "),
        )
    };

    MonthlySimulationViewData {
        month_number: output.month.number(),
        inflow_total: changes.positive.to_string(),
        outflow_total: changes.negative.to_string(),
        inflows,
        outflows,
        stop_reason,
    }
}

fn account_slices(
    accounts: &[property_simulation_application::CashFlowAccountChanges],
    inflow: bool,
) -> Vec<CashFlowChartSlice> {
    let values: Vec<_> = accounts
        .iter()
        .filter_map(|item| {
            let amount = if inflow { item.positive } else { item.negative };
            (!amount.is_zero()).then_some((item, amount, amount_magnitude(amount)))
        })
        .collect();
    let total = values
        .iter()
        .map(|(_, _, magnitude)| magnitude)
        .sum::<f64>();
    values
        .into_iter()
        .map(|(item, amount, magnitude)| CashFlowChartSlice {
            label: item.account_name.clone(),
            category: view_category(item.category),
            amount: amount.to_string(),
            share: (magnitude / total) as f32,
        })
        .collect()
}

fn amount_magnitude(amount: Money) -> f64 {
    amount.coefficient().unsigned_abs() as f64 / 10_f64.powi(i32::from(amount.scale()))
}

const fn view_category(category: CashFlowCategory) -> CashFlowViewCategory {
    match category {
        CashFlowCategory::Operating => CashFlowViewCategory::Operating,
        CashFlowCategory::Investing => CashFlowViewCategory::Investing,
        CashFlowCategory::Financing => CashFlowViewCategory::Financing,
    }
}

fn stop_condition_label(condition: &StopCondition) -> String {
    match condition {
        StopCondition::AfterMonths(months) => format!("Completed {months} months"),
        StopCondition::AllLoansPaidOff => "All configured loans are paid off".to_owned(),
    }
}
