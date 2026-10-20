use super::validation::fatal_run_error;
use super::*;
use property_simulation_accounting::Ledger;
use property_simulation_domain::YearMonth;

pub(super) fn determine_stop_reason(
    run: &mut SimulationRun,
    period: YearMonth,
    ledger: &Ledger,
    identity: Option<RunIdentity>,
) -> Result<Option<StopReason>, RunControlError> {
    let reason = if run.config.end == Some(period) {
        Some(StopReason::EndMonth(period))
    } else if run.config.stop_conditions.is_empty() {
        None
    } else {
        let accounts = ledger.cash_accounts().collect::<Vec<_>>();
        let projection = match ledger.validate_projection(period, period, &accounts) {
            Ok(projection) => projection,
            Err(error) => {
                run.status = RunStatus::Failed;
                return Err(fatal_run_error(
                    crate::FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    identity,
                    None,
                    error.into(),
                ));
            }
        };
        let projection = match projection.project() {
            Ok(projection) => projection,
            Err(error) => {
                run.status = RunStatus::Failed;
                return Err(fatal_run_error(
                    crate::FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    identity,
                    None,
                    error.into(),
                ));
            }
        };
        let change = match projection.cash_flow_statement.totals.net_change() {
            Ok(change) => change,
            Err(error) => {
                run.status = RunStatus::Failed;
                return Err(fatal_run_error(
                    crate::FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    identity,
                    None,
                    error.into(),
                ));
            }
        };
        let mut satisfied = None;
        for (index, condition) in run.config.stop_conditions.iter().enumerate() {
            let StopCondition::MonthlyNetCashChangeAtMost(threshold) = condition;
            let within_threshold = match change.checked_cmp(*threshold) {
                Ok(ordering) => ordering != std::cmp::Ordering::Greater,
                Err(error) => {
                    run.status = RunStatus::Failed;
                    return Err(fatal_run_error(
                        crate::FatalSimulationPhase::ClosingStateDerivation,
                        period,
                        identity,
                        None,
                        error.into(),
                    ));
                }
            };
            if within_threshold {
                satisfied = Some(StopReason::Condition(index));
                break;
            }
        }
        satisfied
    };
    Ok(reason)
}
