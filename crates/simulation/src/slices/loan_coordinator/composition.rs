use super::super::loan::{LoanMonthCommand, LoanMonthInput};
use super::month::prepare_loan_month;
use crate::{
    FatalSimulationFailure, FatalSimulationPhase, MonthlyResult, OutputIdFactory, SimulationError,
    SliceOutput, ValidatedInput, activity::ActivitySample,
    coordinator::coordinate_month_allow_empty_with_transactions,
};
use property_simulation_accounting::BalanceSide;
use property_simulation_domain::Money;

/// Compose ordinary Slice outputs and loan instances into one monthly result and ledger.
/// Loan continuation states, events, postings, and cash-flow contributions are retained
/// in the same result that a caller saves as its month snapshot.
#[allow(clippy::too_many_arguments)]
pub fn coordinate_month_with_loans(
    input: &ValidatedInput,
    opening_ledger: &property_simulation_accounting::Ledger,
    outputs: Vec<SliceOutput>,
    samples: Vec<ActivitySample<crate::ActivitySampleValue>>,
    trial_id: property_simulation_domain::TrialId,
    branch_id: property_simulation_domain::BranchId,
    ids: &dyn OutputIdFactory,
    loans: Vec<LoanMonthInput>,
) -> Result<(MonthlyResult, property_simulation_accounting::Ledger), SimulationError> {
    let mut seen = std::collections::BTreeSet::new();
    let mut liability_accounts = std::collections::BTreeSet::new();
    let mut loan_outputs = Vec::with_capacity(loans.len());
    let mut loan_transactions = Vec::new();
    for loan_input in loans {
        if !seen.insert(loan_input.slice.instance_id)
            || !liability_accounts.insert(loan_input.slice.liability_account)
        {
            return Err(SimulationError::InvalidProposal(
                "monthly loan activity IDs and liability accounts must be unique",
            ));
        }
        let output = prepare_loan_month(
            &loan_input.slice,
            LoanMonthCommand {
                period: input.frozen_input().period,
                loan: loan_input.loan,
                update: loan_input.update,
                opening_ledger,
                trial_id,
                branch_id,
                ids,
            },
        )?;
        loan_transactions.extend(output.transactions.iter().cloned());
        loan_outputs.push(output);
    }
    let (mut result, ledger) = coordinate_month_allow_empty_with_transactions(
        input,
        opening_ledger,
        outputs,
        samples,
        trial_id,
        branch_id,
        ids,
        &loan_transactions,
    )?;
    for mut output in loan_outputs {
        let (side, balance) = ledger.balance(output.liability_account).map_err(|source| {
            SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                output.period,
                Some(trial_id),
                Some(branch_id),
                Some(output.instance_id),
                source.into(),
            ))
        })?;
        let expected = output.closing_loan.map_or_else(
            || Money::from_parts(0, 0, ledger.currency()),
            |loan| Ok(loan.opening_balance),
        )?;
        if side != BalanceSide::Credit || balance != expected {
            return Err(SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                output.period,
                Some(trial_id),
                Some(branch_id),
                Some(output.instance_id),
                SimulationError::InvalidProposal(
                    "loan closing balance does not reconcile to its liability after the monthly batch",
                ),
            )));
        }
        output.ledger = ledger.clone();
        let instance_id = output.instance_id;
        result = result.attach_loan_month(output).map_err(|source| {
            SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::MonthlyResultConstruction,
                input.frozen_input().period,
                Some(trial_id),
                Some(branch_id),
                Some(instance_id),
                source,
            ))
        })?;
    }
    // Direct callers may not go through SimulationRun's loan support preflight.
    // Refuse to return a committed-looking month whose category totals cannot fit Money.
    result.cash_flow_totals()?;
    Ok((result, ledger))
}
