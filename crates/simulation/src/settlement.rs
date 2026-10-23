use std::collections::{BTreeMap, BTreeSet};

use property_simulation_accounting::{LedgerTransaction, LedgerTransactionSource};
use property_simulation_domain::Money;

use crate::proposals::identify_proposal_for;
use crate::validation::LedgerRead;
use crate::{
    FatalSimulationFailure, FatalSimulationPhase, IdentifiedProposal, LoanCalculation,
    LoanContinuation, LoanEvent, LoanStateEffect, MonthlyCashFlowContribution, MonthlyInput,
    MonthlyResult, OutputIdAllocator, SimulationError,
};
use property_simulation_models::{RandomSource, Sampler, UtilityUsageDistribution};

/// Internal services used by the shared month evaluation pipeline.
struct MonthlyRuntime<'a, R, US, LC> {
    pub utility_sampler: &'a mut US,
    pub random: &'a mut R,
    pub loan_calculation: &'a LC,
    pub ids: &'a dyn OutputIdAllocator,
    pub repair_history: Option<crate::RepairHistory>,
    /// Independently derived streams keyed by activity, Slice, month and purpose.
    pub repair_occurrence_random: &'a mut BTreeMap<crate::SliceRandomInputKey, R>,
    pub repair_cost_random: &'a mut BTreeMap<crate::SliceRandomInputKey, R>,
}

/// Runtime services for the explicit RepairOccurrence and RepairCost slices.
/// Caller-owned services for one synchronous calculation using the explicit
/// RepairOccurrence and RepairCost activities.
pub struct SliceMonthlyRuntime<'a, R, US, LC> {
    pub utility_sampler: &'a mut US,
    pub random: &'a mut R,
    pub loan_calculation: &'a LC,
    pub ids: &'a dyn OutputIdAllocator,
    pub repair_history: Option<crate::RepairHistory>,
    /// Independently derived streams keyed by activity, Slice, month and purpose.
    pub repair_occurrence_random: &'a mut BTreeMap<crate::SliceRandomInputKey, R>,
    pub repair_cost_random: &'a mut BTreeMap<crate::SliceRandomInputKey, R>,
}

/// Calculate and settle one month containing explicit activity configurations.
pub fn calculate_monthly_slice_result<R, US, LC>(
    input: &MonthlyInput<'_>,
    ledger_read: &impl LedgerRead,
    runtime: &mut SliceMonthlyRuntime<'_, R, US, LC>,
) -> Result<MonthlyResult, SimulationError>
where
    R: RandomSource,
    US: Sampler<UtilityUsageDistribution>,
    LC: LoanCalculation,
{
    let mut monthly_runtime = MonthlyRuntime {
        utility_sampler: runtime.utility_sampler,
        random: runtime.random,
        loan_calculation: runtime.loan_calculation,
        ids: runtime.ids,
        repair_history: runtime.repair_history,
        repair_occurrence_random: runtime.repair_occurrence_random,
        repair_cost_random: runtime.repair_cost_random,
    };
    calculate_monthly_slice_result_inner(input, ledger_read, &mut monthly_runtime)
}

fn calculate_monthly_slice_result_inner<R, US, LC>(
    input: &MonthlyInput<'_>,
    ledger_read: &impl LedgerRead,
    runtime: &mut MonthlyRuntime<'_, R, US, LC>,
) -> Result<MonthlyResult, SimulationError>
where
    R: RandomSource,
    US: Sampler<UtilityUsageDistribution>,
    LC: LoanCalculation,
{
    let period = input.frozen_input.period;
    if ledger_read.currency() != input.opening_ledger.currency() {
        return Err(SimulationError::InvalidInput(
            "Ledger reader currency does not match opening Ledger",
        ));
    }
    for activity in input.activities {
        match activity.configuration {
            crate::ActivityConfiguration::RepairOccurrence(_)
                if !runtime.repair_occurrence_random.contains_key(
                    &crate::SliceRandomInputKey {
                        activity_id: activity.id,
                        slice_id: activity.slice_id,
                        period,
                        purpose: "repair.occurrence",
                    },
                ) =>
            {
                return Err(SimulationError::InvalidInput(
                    "repair occurrence random input is missing for an active instance",
                ));
            }
            crate::ActivityConfiguration::RepairCost(_)
                if !runtime
                    .repair_cost_random
                    .contains_key(&crate::SliceRandomInputKey {
                        activity_id: activity.id,
                        slice_id: activity.slice_id,
                        period,
                        purpose: "repair.cost",
                    }) =>
            {
                return Err(SimulationError::InvalidInput(
                    "repair cost random input is missing for an active instance",
                ));
            }
            _ => {}
        }
    }
    // Complete all instance, account, and loan validation before sampling or
    // invoking any Slice transition implementation.
    let prepared = crate::monthly::prepare_monthly_slices(
        input,
        runtime.repair_history,
        ledger_read,
        runtime.loan_calculation,
    )?;
    let loan_activities = input
        .activities
        .iter()
        .filter(|a| matches!(a.configuration, crate::ActivityConfiguration::Loan(_)))
        .map(|a| (a.id, a))
        .collect::<BTreeMap<_, _>>();
    let continuations = input
        .loan_continuations
        .iter()
        .map(|c| (c.instance_id, c))
        .collect::<BTreeMap<_, _>>();
    let slices = loan_activities
        .iter()
        .map(|(&id, activity)| {
            let crate::ActivityConfiguration::Loan(config) = &activity.configuration else {
                unreachable!("loan activity map contains only loan configurations")
            };
            (id, crate::monthly::loan_slice(activity, *config))
        })
        .collect::<BTreeMap<_, _>>();
    let loan_outputs = prepared.loan_outputs().clone();

    let (slice_outputs, samples) = crate::monthly::evaluate_prepared_monthly_slices(
        prepared,
        runtime.repair_history,
        runtime.utility_sampler,
        runtime.random,
        runtime.repair_occurrence_random,
        runtime.repair_cost_random,
    )?;
    let mut proposal = crate::monthly::compose_prepared_monthly_slices(slice_outputs, runtime.ids)?;

    let mut loan_events = Vec::new();
    let mut loan_effects = Vec::new();
    let mut closing_loans = Vec::new();
    let mut loan_proposals = Vec::new();
    for (instance_id, activity) in loan_activities {
        let continuation = continuations[&instance_id];
        let mut events = Vec::new();
        let mut effects = Vec::new();
        let proposal_for_loan = match continuation.loan {
            Some(loan) => {
                let output = &loan_outputs[&instance_id];
                let calculation = output.result;
                let identified =
                    identify_proposal_for(output.proposal.clone(), runtime.ids, activity, period)?;
                for event in &identified.events {
                    events.push(LoanEvent {
                        id: event.id,
                        instance_id,
                        slice_id: activity.slice_id,
                        period,
                        key: event.key.clone(),
                        previous_balance: Some(loan.opening_balance),
                        updated_balance: Some(calculation.closing_balance),
                        previous_remaining_months: Some(loan.remaining_months),
                        updated_remaining_months: Some(calculation.remaining_months),
                        previous_annual_rate: Some(loan.annual_rate),
                        updated_annual_rate: (!calculation.closing_balance.is_zero())
                            .then_some(loan.annual_rate),
                    });
                }
                let updated = output.state_effect.updated;
                for event in &events {
                    effects.push(LoanStateEffect {
                        id: runtime.ids.effect_id(period, instance_id, "loan_payment"),
                        instance_id,
                        slice_id: activity.slice_id,
                        period,
                        previous: Some(loan),
                        updated,
                        source_event_id: event.id,
                    });
                }
                closing_loans.push(LoanContinuation {
                    instance_id,
                    slice_id: activity.slice_id,
                    loan: updated,
                });
                identified
            }
            None => {
                closing_loans.push(continuation.clone());
                IdentifiedProposal::default()
            }
        };
        loan_events.extend(events);
        loan_effects.extend(effects);
        loan_proposals.push(proposal_for_loan);
    }
    proposal = crate::merge_proposals(std::iter::once(proposal).chain(loan_proposals))?;
    let loan_event_ids = loan_events
        .iter()
        .map(|event| event.id)
        .collect::<BTreeSet<_>>();
    proposal
        .events
        .retain(|event| !loan_event_ids.contains(&event.id));
    let mut lifecycle_changes = Vec::new();
    for activity in input.activities {
        match &activity.configuration {
            crate::ActivityConfiguration::RepairOccurrence(config) => {
                let occurred = samples.iter().any(|sample| {
                    sample.instance_id == activity.id
                        && sample.value
                            == crate::ActivitySampleValue::RepairOccurrence(
                                property_simulation_models::RepairOccurrenceSample(true),
                            )
                });
                if occurred {
                    let event = proposal
                        .events()
                        .iter()
                        .find(|event| {
                            event.instance_id == activity.id && event.key == "repair_occurrence"
                        })
                        .ok_or(SimulationError::InvalidProposal(
                            "repair occurrence sample has no event",
                        ))?;
                    let delay = i32::try_from(config.cost_template.delay_months).map_err(|_| {
                        SimulationError::InvalidInput("repair cost delay is out of range")
                    })?;
                    let effective_period = period.checked_add_months(delay)?;
                    let instance = crate::ActivityInstance {
                        id: runtime
                            .ids
                            .activity_instance_id(period, activity.id, "repair_cost"),
                        slice_id: activity.slice_id,
                        configuration: crate::ActivityConfiguration::RepairCost(
                            crate::RepairCostConfig {
                                source_event_id: event.id,
                                source_instance_id: activity.id,
                                source_slice_id: activity.slice_id,
                                occurrence_period: period,
                                delay_months: config.cost_template.delay_months,
                                effective_period,
                                maximum: config.cost_template.maximum,
                                expense_account: config.cost_template.expense_account,
                            },
                        ),
                    };
                    lifecycle_changes.push(crate::InstanceLifecycleChange {
                        effective_period,
                        operation: crate::InstanceLifecycleOperation::Add(instance),
                    });
                }
            }
            crate::ActivityConfiguration::RepairCost(_) => {
                lifecycle_changes.push(crate::InstanceLifecycleChange {
                    effective_period: period,
                    operation: crate::InstanceLifecycleOperation::Complete {
                        instance_id: activity.id,
                    },
                })
            }
            _ => {}
        }
    }
    let mut events = proposal
        .events()
        .iter()
        .map(|e| (e.id, (e.instance_id, e.slice_id)))
        .collect::<BTreeMap<_, _>>();
    events.extend(
        loan_events
            .iter()
            .map(|event| (event.id, (event.instance_id, event.slice_id))),
    );
    for activity in input.activities {
        if let crate::ActivityConfiguration::RepairCost(config) = &activity.configuration {
            events.insert(
                config.source_event_id,
                (config.source_instance_id, config.source_slice_id),
            );
        }
    }
    let mut transactions = Vec::new();
    for p in proposal.transactions() {
        transactions.push(
            LedgerTransaction::new(p.id, p.period, p.entries.clone()).with_source(
                LedgerTransactionSource {
                    instance_id: p.instance_id,
                    slice_id: p.slice_id,
                    output_key: p.key.clone(),
                    source_event_id: p.source_event_id,
                },
            ),
        );
    }
    let mut candidate = input.opening_ledger.clone();
    candidate.post_batch(transactions.clone()).map_err(|e| {
        SimulationError::Fatal(FatalSimulationFailure::new(
            FatalSimulationPhase::LedgerCommit,
            period,
            None,
            e.into(),
        ))
    })?;
    for continuation in &closing_loans {
        let slice = &slices[&continuation.instance_id];
        let expected_balance = continuation
            .loan
            .map(|loan| loan.opening_balance.checked_neg())
            .transpose()
            .map_err(|error| {
                SimulationError::Fatal(FatalSimulationFailure::new(
                    FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    Some(continuation.instance_id),
                    error.into(),
                ))
            })?
            .unwrap_or_else(|| Money::new(0.into(), candidate.currency()));
        let actual_balance =
            signed_balance(&candidate, slice.liability_account).map_err(|error| {
                SimulationError::Fatal(FatalSimulationFailure::new(
                    FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    Some(continuation.instance_id),
                    error,
                ))
            })?;
        if actual_balance != expected_balance {
            return Err(SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                period,
                Some(continuation.instance_id),
                SimulationError::InvalidProposal(
                    "closing loan continuation does not match its Ledger liability balance",
                ),
            )));
        }
    }
    let transaction_ids = transactions
        .iter()
        .map(LedgerTransaction::id)
        .collect::<BTreeSet<_>>();
    let cash_flows = candidate
        .cash_flow_contributions(period, period)
        .map_err(|e| {
            SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                period,
                None,
                e.into(),
            ))
        })?
        .into_iter()
        .filter(|c| transaction_ids.contains(&c.transaction_id))
        .map(|c| {
            let source = c.source.ok_or_else(|| {
                SimulationError::Fatal(FatalSimulationFailure::new(
                    FatalSimulationPhase::ClosingStateDerivation,
                    period,
                    None,
                    SimulationError::InvalidProposal(
                        "posted monthly transaction is missing source attribution",
                    ),
                ))
            })?;
            Ok(MonthlyCashFlowContribution {
                period: c.period,
                instance_id: source.instance_id,
                slice_id: source.slice_id,
                output_key: source.output_key,
                transaction_id: c.transaction_id,
                category: c.category,
                amount: c.amount,
            })
        })
        .collect::<Result<Vec<_>, SimulationError>>()?;
    let mut closing_state = input.opening_state.clone();
    closing_state.cash =
        signed_balance(&candidate, input.frozen_input.cash_account).map_err(|error| {
            SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::ClosingStateDerivation,
                period,
                None,
                error,
            ))
        })?;
    for effect in proposal.effects() {
        if effect.target.as_str() != "property.condition" {
            return Err(SimulationError::InvalidProposal(
                "monthly result cannot apply an unknown state effect",
            ));
        }
        closing_state.condition = effect.condition;
    }
    for effect in &loan_effects {
        if !loan_events.iter().any(|e| e.id == effect.source_event_id) {
            return Err(SimulationError::InvalidProposal(
                "loan effect has no source event",
            ));
        }
    }
    // Ensure transaction/event source references survived merging before constructing the success value.
    if proposal.transactions().iter().any(|t| t.source_event_id.is_some_and(|id| {
        events.get(&id) != Some(&(t.instance_id, t.slice_id))
            && !input.activities.iter().any(|activity| {
                activity.id == t.instance_id && matches!(&activity.configuration,
                    crate::ActivityConfiguration::RepairCost(config)
                        if config.source_event_id == id
                            && events.get(&id) == Some(&(config.source_instance_id, config.source_slice_id)))
            })
    })) {
        return Err(SimulationError::InvalidProposal(
            "transaction source event is not in the monthly proposal",
        ));
    }
    MonthlyResult::from_settled_parts_with_lifecycle(
        period,
        input.opening_state.clone(),
        *input.frozen_input,
        input.activities.to_vec(),
        samples,
        closing_state,
        proposal,
        lifecycle_changes,
        transactions,
        cash_flows,
        input.loan_continuations.to_vec(),
        closing_loans,
        loan_events,
        loan_effects,
    )
    .map_err(|e| {
        SimulationError::Fatal(FatalSimulationFailure::new(
            FatalSimulationPhase::MonthlyResultConstruction,
            period,
            None,
            e,
        ))
    })
}

fn signed_balance(
    ledger: &property_simulation_accounting::Ledger,
    account: property_simulation_domain::AccountId,
) -> Result<Money, SimulationError> {
    let (side, amount) = ledger.balance(account)?;
    match side {
        property_simulation_accounting::BalanceSide::Debit => Ok(amount),
        property_simulation_accounting::BalanceSide::Credit => {
            amount.checked_neg().map_err(SimulationError::from)
        }
    }
}
