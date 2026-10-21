use super::{RunControlError, RunIdentity, RunUpdate, RunUpdateValue};
use crate::{LoanContinuation, LoanRunUpdate, MonthlyResult, SimulationError};
use property_simulation_accounting::BalanceSide;
use property_simulation_domain::{Money, YearMonth};

pub(super) fn fatal_run_error(
    phase: crate::FatalSimulationPhase,
    period: YearMonth,
    identity: Option<RunIdentity>,
    activity_instance_id: Option<property_simulation_domain::ActivityInstanceId>,
    source: SimulationError,
) -> RunControlError {
    RunControlError::Simulation(SimulationError::Fatal(
        crate::FatalSimulationFailure::with_run_identity(
            phase,
            period,
            identity.map(|identity| identity.trial_id),
            identity.map(|identity| identity.branch_id),
            activity_instance_id,
            source,
        ),
    ))
}

pub(super) fn has_duplicate_activity_ids(activities: &[crate::ActivityInstance]) -> bool {
    let mut ids = std::collections::BTreeSet::new();
    activities.iter().any(|activity| !ids.insert(activity.id))
}

pub(super) fn additional_cash_flow_support(
    loans: &[LoanContinuation],
    loan_updates: &[LoanRunUpdate],
    updates: &[RunUpdate],
    currency: property_simulation_domain::Currency,
) -> Result<Vec<(property_simulation_accounting::CashFlowCategory, Money)>, SimulationError> {
    let mut bounds = Vec::new();
    for continuation in loans {
        let update = loan_updates
            .iter()
            .find(|update| update.instance_id == continuation.instance_id);
        if let Some(update) = update {
            let principal_change = update.update.principal_change();
            if !principal_change.is_zero() {
                bounds.push((
                    property_simulation_accounting::CashFlowCategory::Financing,
                    principal_change.abs()?,
                ));
            }
        }
        let loan = update
            .map(|update| update.update.updated_loan())
            .unwrap_or(continuation.loan);
        if let Some(loan) = loan {
            let month = loan.calculate_month().map_err(|_| {
                SimulationError::InvalidInput("loan cash-flow support cannot be calculated")
            })?;
            bounds.extend([
                (
                    property_simulation_accounting::CashFlowCategory::Operating,
                    month.interest,
                ),
                (
                    property_simulation_accounting::CashFlowCategory::Financing,
                    month.principal,
                ),
            ]);
        }
    }
    for update in updates {
        let RunUpdateValue::StateAdjustment { transaction, .. } = &update.value else {
            continue;
        };
        let mut by_category = std::collections::BTreeMap::<
            property_simulation_accounting::CashFlowCategory,
            Vec<Money>,
        >::new();
        for entry in transaction.entries() {
            let Some(category) = entry.cash_flow_category() else {
                continue;
            };
            let amount = match entry.side() {
                BalanceSide::Debit => entry.amount(),
                BalanceSide::Credit => entry.amount().checked_neg()?,
            };
            by_category.entry(category).or_default().push(amount);
        }
        for (category, values) in by_category {
            let net = crate::types::exact_signed_sum(values, currency)?;
            bounds.push((category, net.abs()?));
        }
    }
    Ok(bounds)
}

pub(super) fn loan_output_matches(
    activities: &[crate::ActivityInstance],
    prior_loans: &[LoanContinuation],
    updates: &[LoanRunUpdate],
    period: YearMonth,
    result: &MonthlyResult,
) -> bool {
    let expected: std::collections::BTreeMap<_, _> = activities
        .iter()
        .filter(|activity| activity.configuration.sample_kind() == crate::ActivitySampleKind::Loan)
        .map(|activity| (activity.id, activity.slice_id))
        .collect();
    let actual: std::collections::BTreeMap<_, _> = result
        .loan_continuations()
        .iter()
        .map(|loan| (loan.instance_id, loan))
        .collect();
    let prior: std::collections::BTreeMap<_, _> = prior_loans
        .iter()
        .map(|loan| (loan.instance_id, loan))
        .collect();
    if actual.len() != expected.len() || actual.len() != result.loan_continuations().len() {
        return false;
    }
    for (instance_id, slice_id) in &expected {
        let (Some(before), Some(after)) = (prior.get(instance_id), actual.get(instance_id)) else {
            return false;
        };
        if before.slice_id != *slice_id || after.slice_id != *slice_id {
            return false;
        }
        let update = updates
            .iter()
            .find(|update| update.instance_id == *instance_id);
        let mut loan = match (update, before.loan) {
            (Some(update), Some(current)) if update.update.expected_loan() == current => {
                update.update.updated_loan()
            }
            (None, current) => current,
            _ => return false,
        };
        let previous = before.loan;
        let should_effect = loan.is_some() || update.is_some();
        if let Some(active) = loan {
            let Ok(month) = active.calculate_month() else {
                return false;
            };
            loan = if month.closing_balance.is_zero() {
                None
            } else {
                Some(property_simulation_models::FixedRateLoan {
                    opening_balance: month.closing_balance,
                    remaining_months: month.remaining_months,
                    annual_rate: active.annual_rate,
                    scheduled_payment: Some(month.scheduled_payment),
                })
            };
        }
        if after.loan != loan {
            return false;
        }
        let instance_events: Vec<_> = result
            .loan_events()
            .iter()
            .filter(|event| event.instance_id == *instance_id && event.period == period)
            .collect();
        let payment_loan = update.map_or(previous, |u| u.update.updated_loan());
        let expected_event_count =
            usize::from(update.is_some()) + usize::from(payment_loan.is_some());
        if instance_events.len() != expected_event_count
            || instance_events
                .iter()
                .any(|event| !matches!(event.key.as_str(), "loan_terms_update" | "loan_payment"))
        {
            return false;
        }
        let update_event = instance_events
            .iter()
            .copied()
            .find(|event| event.key == "loan_terms_update");
        if let Some(update) = update {
            let Some(event) = update_event else {
                return false;
            };
            let (Some(previous_loan), updated_loan) = (previous, update.update.updated_loan())
            else {
                return false;
            };
            let updated_balance = updated_loan.map_or_else(
                || Money::from_parts(0, 0, previous_loan.opening_balance.currency()),
                |loan| Ok(loan.opening_balance),
            );
            let Ok(updated_balance) = updated_balance else {
                return false;
            };
            if event.slice_id != *slice_id
                || event.previous_balance != Some(previous_loan.opening_balance)
                || event.updated_balance != Some(updated_balance)
                || event.previous_remaining_months != Some(previous_loan.remaining_months)
                || event.updated_remaining_months != updated_loan.map(|loan| loan.remaining_months)
                || event.previous_annual_rate != Some(previous_loan.annual_rate)
                || event.updated_annual_rate != updated_loan.map(|loan| loan.annual_rate)
            {
                return false;
            }
        }
        let payment_event = instance_events
            .iter()
            .copied()
            .find(|event| event.key == "loan_payment");
        if let Some(payment_loan) = payment_loan {
            let (Some(event), Ok(month)) = (payment_event, payment_loan.calculate_month()) else {
                return false;
            };
            if event.slice_id != *slice_id
                || event.previous_balance != Some(payment_loan.opening_balance)
                || event.updated_balance != Some(month.closing_balance)
                || event.previous_remaining_months != Some(payment_loan.remaining_months)
                || event.updated_remaining_months != Some(month.remaining_months)
                || event.previous_annual_rate != Some(payment_loan.annual_rate)
                || event.updated_annual_rate != Some(payment_loan.annual_rate)
            {
                return false;
            }
        }
        let effects: Vec<_> = result
            .loan_effects()
            .iter()
            .filter(|effect| effect.instance_id == *instance_id && effect.period == period)
            .collect();
        if effects.len() != usize::from(should_effect) {
            return false;
        }
        if let Some(effect) = effects.first() {
            let expected_source = payment_event.or(update_event).map(|event| event.id);
            if effect.slice_id != *slice_id
                || effect.previous != previous
                || effect.updated != loan
                || expected_source != Some(effect.source_event_id)
            {
                return false;
            }
        }
    }
    result
        .loan_events()
        .iter()
        .all(|event| event.period != period || expected.contains_key(&event.instance_id))
        && result
            .loan_effects()
            .iter()
            .all(|effect| effect.period != period || expected.contains_key(&effect.instance_id))
}
