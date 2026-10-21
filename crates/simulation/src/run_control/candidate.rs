use super::validation::{additional_cash_flow_support, has_duplicate_activity_ids};
use super::*;
use crate::{validate_input, validation::validate_cashflow_support_with_additional};
use property_simulation_accounting::BalanceSide;
use property_simulation_domain::EffectId;
use property_simulation_models::RepairCostDistribution;

pub(super) struct PreparedMonth {
    pub(super) input: FrozenInput,
    pub(super) candidate_ledger: Ledger,
    pub(super) activities: Vec<crate::ActivityInstance>,
    pub(super) loan_updates: Vec<LoanRunUpdate>,
    pub(super) input_effects: Vec<crate::RunInputEffect>,
    pub(super) candidate_loans: Vec<LoanContinuation>,
    pub(super) validated: ValidatedInput,
}

pub(super) fn prepare_month(
    run: &SimulationRun,
    updates: &[RunUpdate],
) -> Result<PreparedMonth, RunControlError> {
    let mut input = run.frozen_input;
    input.period = run.next_month;
    let mut candidate_ledger = run.ledger.clone();
    let mut state = run.opening_state.clone();
    let mut activities = run.activities.clone();
    let mut loan_updates = Vec::new();
    let mut input_effects = Vec::new();
    let mut updated_loan_ids = std::collections::BTreeSet::new();
    for update in updates {
        if update.source.trim().is_empty() || update.effective_month != run.next_month {
            return Err(RunControlError::InvalidUpdate(
                "update source and effective month are required",
            ));
        }
        match &update.value {
            RunUpdateValue::Rent(value) => input.rent = *value,
            RunUpdateValue::RepairCostLimit(value) => input.repair_cost_limit = *value,
            RunUpdateValue::Condition(value) => state.condition = *value,
            RunUpdateValue::Activities(value) => activities = value.clone(),
            RunUpdateValue::LoanTerms(value) => {
                let Some(current) = run
                    .loan_continuations
                    .iter()
                    .find(|loan| loan.instance_id == value.instance_id)
                else {
                    return Err(RunControlError::InvalidUpdate(
                        "loan update references an unknown activity",
                    ));
                };
                if current.loan != Some(value.update.expected_loan())
                    || value.update.effective_period() != run.next_month
                    || !updated_loan_ids.insert(value.instance_id)
                {
                    return Err(RunControlError::InvalidUpdate(
                        "loan update must match the current loan and be unique",
                    ));
                }
                loan_updates.push((**value).clone());
            }
            RunUpdateValue::StateAdjustment {
                state: updated,
                transaction,
            } => {
                let previous_state = state.clone();
                if transaction.period() != run.next_month {
                    return Err(RunControlError::InvalidUpdate(
                        "state adjustment must post in its effective month",
                    ));
                }
                candidate_ledger.post(transaction.clone()).map_err(|_| {
                    RunControlError::InvalidUpdate("state adjustment transaction is invalid")
                })?;
                let (side, amount) =
                    candidate_ledger.balance(input.cash_account).map_err(|_| {
                        RunControlError::InvalidUpdate(
                            "state adjustment cash balance is unavailable",
                        )
                    })?;
                let cash = if side == BalanceSide::Debit {
                    amount
                } else {
                    amount.checked_neg().map_err(|_| {
                        RunControlError::InvalidUpdate(
                            "state adjustment cash balance cannot be represented",
                        )
                    })?
                };
                if updated.cash != cash {
                    return Err(RunControlError::InvalidUpdate(
                        "state adjustment cash must match its ledger posting",
                    ));
                }
                input_effects.push(crate::RunInputEffect {
                    id: EffectId::from_bytes(*transaction.id().as_bytes()),
                    source: update.source.clone(),
                    period: run.next_month,
                    previous: previous_state,
                    updated: updated.clone(),
                    transaction_id: transaction.id(),
                });
                state = updated.clone();
            }
        }
    }
    if has_duplicate_activity_ids(&activities) {
        return Err(RunControlError::InvalidUpdate(
            "active activity instance IDs must be unique",
        ));
    }
    let loan_activity_map: std::collections::BTreeMap<_, _> = activities
        .iter()
        .filter(|activity| activity.configuration.sample_kind() == crate::ActivitySampleKind::Loan)
        .map(|activity| (activity.id, (activity.slice_id, activity.version)))
        .collect();
    let prior_loans: std::collections::BTreeMap<_, _> = run
        .loan_continuations
        .iter()
        .map(|loan| (loan.instance_id, loan))
        .collect();
    let mut candidate_loans = Vec::with_capacity(loan_activity_map.len());
    for (instance_id, (slice_id, version)) in &loan_activity_map {
        let Some(loan) = prior_loans.get(instance_id) else {
            return Err(RunControlError::InvalidUpdate(
                "a new loan activity requires an explicit opening loan state",
            ));
        };
        if loan.slice_id != *slice_id || loan.model_version != *version {
            return Err(RunControlError::InvalidUpdate(
                "loan activity identity cannot change while it is active",
            ));
        }
        candidate_loans.push((*loan).clone());
    }
    if run
        .loan_continuations
        .iter()
        .any(|loan| !loan_activity_map.contains_key(&loan.instance_id) && loan.loan.is_some())
    {
        return Err(RunControlError::InvalidUpdate(
            "an active loan cannot be removed from the activity set",
        ));
    }
    let distribution = RepairCostDistribution {
        maximum: input.repair_cost_limit,
    };
    let validated = validate_input(&input, &distribution, &candidate_ledger, &state)
        .map_err(RunControlError::Input)?;
    for activity in &activities {
        crate::validation::validate_activity_configuration(activity, &input, &candidate_ledger)
            .map_err(RunControlError::InvalidExecutionInput)?;
    }
    let additional_support = additional_cash_flow_support(
        &candidate_loans,
        &loan_updates,
        updates,
        candidate_ledger.currency(),
    )
    .map_err(RunControlError::InvalidExecutionInput)?;
    validate_cashflow_support_with_additional(&input, &activities, additional_support)
        .map_err(RunControlError::InvalidExecutionInput)?;
    Ok(PreparedMonth {
        input,
        candidate_ledger,
        activities,
        loan_updates,
        input_effects,
        candidate_loans,
        validated,
    })
}
