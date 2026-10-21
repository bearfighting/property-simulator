use super::candidate::PreparedMonth;
use super::*;

impl SimulationRun {
    pub(super) fn execute_prepared_month(
        &mut self,
        prepared: &PreparedMonth,
        executor: &mut dyn SingleMonthExecutor,
    ) -> Result<(MonthlyResult, Ledger), RunControlError> {
        if let Err(error) = executor.validate_inputs(
            &prepared.validated,
            &prepared.candidate_ledger,
            &prepared.activities,
        ) {
            if matches!(error, SimulationError::Fatal(_)) {
                self.status = RunStatus::Failed;
                return Err(RunControlError::Simulation(error));
            }
            return Err(RunControlError::InvalidExecutionInput(error));
        }
        if let Err(error) = executor.validate_loan_context(
            &prepared.validated,
            &prepared.candidate_ledger,
            &prepared.candidate_loans,
            &prepared.loan_updates,
        ) {
            if matches!(error, SimulationError::Fatal(_)) {
                self.status = RunStatus::Failed;
                return Err(RunControlError::Simulation(error));
            }
            return Err(RunControlError::InvalidExecutionInput(error));
        }

        let requires_random_streams = executor.requires_random_streams();
        if self
            .randomness_mode
            .is_some_and(|previous| previous != requires_random_streams)
        {
            return Err(RunControlError::Randomness(
                "executor randomness requirement changed during the run",
            ));
        }
        if requires_random_streams && self.run_identity.is_none() {
            return Err(RunControlError::Randomness(
                "random streams require a trial identity",
            ));
        }
        self.randomness_mode = Some(requires_random_streams);
        if requires_random_streams && self.random_streams.is_none() {
            let streams = executor
                .create_random_streams(self.run_identity)
                .map_err(|_| RunControlError::Randomness("failed to create random streams"))?;
            self.random_streams = Some(streams.ok_or(RunControlError::Randomness(
                "executor requested random streams but did not create them",
            ))?);
        }
        let random_streams = self.random_streams.as_ref();
        let (result, ledger) = match executor.execute_with_loan_context(
            &prepared.validated,
            &prepared.candidate_ledger,
            &prepared.activities,
            &prepared.candidate_loans,
            &prepared.loan_updates,
            self.run_identity,
            random_streams,
        ) {
            Ok(completed) => completed,
            Err(error) => {
                if matches!(error, SimulationError::Fatal(_)) {
                    self.status = RunStatus::Failed;
                    return Err(RunControlError::Simulation(error));
                }
                // Sample/proposal validation and transaction preflight errors are
                // still before the coordinator's atomic commit boundary.
                return Err(RunControlError::InvalidExecutionInput(error));
            }
        };
        Ok((result, ledger))
    }
}
