use super::candidate::PreparedMonth;
use super::validation::{fatal_run_error, loan_output_matches};
use super::*;
use property_simulation_accounting::{BalanceSide, LedgerTransaction};

impl SimulationRun {
    pub(super) fn validate_month_output(
        &mut self,
        result: TestMonthlyResult,
        ledger: &Ledger,
        prepared: &PreparedMonth,
        updates: &[RunUpdate],
    ) -> Result<(TestMonthlyResult, Option<RunIdentity>), RunControlError> {
        let returned_identity = RunIdentity {
            trial_id: result.trial_id(),
            branch_id: result.branch_id(),
        };
        let diagnostic_identity = Some(self.run_identity.unwrap_or(returned_identity));
        let result = result
            .with_input_updates(updates.to_vec(), prepared.input_effects.clone())
            .map_err(|source| {
                self.status = RunStatus::Failed;
                fatal_run_error(
                    crate::FatalSimulationPhase::MonthlyResultConstruction,
                    self.next_month,
                    diagnostic_identity,
                    None,
                    source,
                )
            })?;
        let opening_transactions: Vec<_> =
            prepared.candidate_ledger.transactions().cloned().collect();
        let returned_transactions: Vec<_> = ledger.transactions().cloned().collect();
        let returned_month_transactions = if returned_transactions.len()
            >= opening_transactions.len()
            && returned_transactions.starts_with(&opening_transactions)
        {
            &returned_transactions[opening_transactions.len()..]
        } else {
            &[]
        };
        let mut reported_transactions: Vec<_> = result
            .proposal()
            .transactions()
            .iter()
            .cloned()
            .map(crate::ProposedTransaction::into_ledger_transaction)
            .collect();
        reported_transactions.sort_by_key(LedgerTransaction::id);
        let mut returned_month_transactions_sorted = returned_month_transactions.to_vec();
        returned_month_transactions_sorted.sort_by_key(LedgerTransaction::id);
        let mut expected_activities = prepared.activities.clone();
        expected_activities.sort_by_key(|activity| activity.id);
        let cash_matches =
            ledger
                .balance(prepared.input.cash_account)
                .ok()
                .and_then(|(side, amount)| {
                    if side == BalanceSide::Debit {
                        Some(amount)
                    } else {
                        amount.checked_neg().ok()
                    }
                })
                == Some(result.closing_state().cash);
        let output_is_consistent = result.period() == self.next_month
            && self
                .run_identity
                .is_none_or(|identity| identity == returned_identity)
            && result.opening_state() == prepared.validated.opening_state()
            && result.frozen_input() == &prepared.input
            && result.activities() == expected_activities
            && loan_output_matches(
                &prepared.activities,
                &prepared.candidate_loans,
                &prepared.loan_updates,
                self.next_month,
                &result,
            )
            && prepared.loan_updates.iter().all(|update| {
                result.loan_events().iter().any(|event| {
                    event.instance_id == update.instance_id
                        && event.period == self.next_month
                        && event.key == "loan_terms_update"
                })
            })
            && cash_matches
            && ledger.chart() == prepared.candidate_ledger.chart()
            && ledger.opening_balances() == prepared.candidate_ledger.opening_balances()
            && ledger
                .cash_accounts()
                .eq(prepared.candidate_ledger.cash_accounts())
            && returned_transactions.starts_with(&opening_transactions)
            && returned_month_transactions
                .iter()
                .all(|transaction| transaction.period() == self.next_month)
            && returned_month_transactions_sorted == reported_transactions;
        if !output_is_consistent {
            self.status = RunStatus::Failed;
            return Err(fatal_run_error(
                crate::FatalSimulationPhase::MonthlyResultConstruction,
                self.next_month,
                diagnostic_identity,
                None,
                SimulationError::InvalidInput(
                    "single-month executor returned an inconsistent result or ledger",
                ),
            ));
        }
        Ok((result, diagnostic_identity))
    }
}
