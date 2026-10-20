use super::candidate;
use super::stopping;
use super::*;

impl SimulationRun {
    pub fn call(
        &mut self,
        command: RunCommand,
        updates: &[RunUpdate],
        executor: &mut dyn SingleMonthExecutor,
    ) -> Result<RunCallResult, RunControlError> {
        if !matches!(self.status, RunStatus::Running { .. }) {
            return Err(RunControlError::InvalidState);
        }
        if command == RunCommand::Stop {
            self.status = RunStatus::Stopped(StopReason::User);
            return Ok(self.response(None));
        }

        let prepared = candidate::prepare_month(self, updates)?;
        let (result, ledger) = self.execute_prepared_month(&prepared, executor)?;
        let (result, diagnostic_identity) =
            self.validate_month_output(result, &ledger, &prepared, updates)?;
        let period = result.period();
        let mut reason =
            stopping::determine_stop_reason(self, period, &ledger, diagnostic_identity)?;
        let next_month = if reason.is_none() {
            match period.checked_add_months(1) {
                Ok(next_month) => Some(next_month),
                Err(_) => {
                    reason = Some(StopReason::CalendarLimit(period));
                    None
                }
            }
        } else {
            None
        };

        self.opening_state = result.closing_state().clone();
        self.frozen_input = prepared.input;
        self.activities = prepared.activities;
        self.loan_continuations = result.loan_continuations().to_vec();
        self.ledger = ledger.clone();
        self.history.extend_from_slice(updates);
        self.latest = Some(MonthSnapshot {
            result: result.clone(),
            ledger,
        });
        self.identity = Some((result.trial_id(), result.branch_id()));
        if let Some(reason) = reason {
            self.status = RunStatus::Stopped(reason);
        } else {
            self.next_month = next_month.expect("an unstopped month has a representable successor");
            self.frozen_input.period = self.next_month;
            self.status = RunStatus::Running {
                next_month: self.next_month,
            };
        }
        Ok(self.response(Some(result)))
    }

    fn response(&self, monthly_result: Option<MonthlyResult>) -> RunCallResult {
        let stop_reason = match self.status {
            RunStatus::Stopped(reason) => Some(reason),
            _ => None,
        };
        RunCallResult {
            monthly_result,
            latest_completed_month: self
                .latest
                .as_ref()
                .map(|snapshot| snapshot.result.period()),
            status: self.status.clone(),
            stop_reason,
        }
    }
}
