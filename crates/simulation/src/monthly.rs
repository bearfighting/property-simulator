use std::collections::{BTreeMap, BTreeSet};

use property_simulation_accounting::CashFlowCategory;
use property_simulation_domain::ActivityInstanceId;
use property_simulation_models::{
    RandomSource, RepairCostDistribution, RepairCostSampler, RepairOccurrenceDistribution,
    RepairOccurrenceSampler, Sampler, UtilityUsageDistribution,
};

use crate::activity::{
    ActivitySample, ActivitySampleValue, SliceOutput, propose_slice, sample_utility,
};
use crate::composition::validate_composition;
use crate::error::{FatalSimulationFailure, FatalSimulationPhase, SimulationError};
use crate::proposals::{identify_proposal, merge_proposals};
use crate::slices::{
    LoanCalculation, LoanMonthOutput, LoanSlice, validate_loan_calculation,
    validate_loan_opening_state,
};
use crate::types::{
    ActivityConfiguration, ActivityInstance, IdentifiedProposal, LoanConfig, MonthlyInput,
    OutputIdAllocator,
};
use crate::validation::{LedgerRead, ValidatedInput};

/// A fully preflighted month. No sampler or Slice transition has run yet.
pub(crate) struct PreparedMonthlySlices<'input, 'data: 'input> {
    monthly_input: &'input MonthlyInput<'data>,
    validated: ValidatedInput,
    loan_outputs: BTreeMap<ActivityInstanceId, LoanMonthOutput>,
}

/// Validate all active instances, shared inputs, and loan continuation/account
/// relationships before the monthly dispatcher samples or evaluates any Slice.
pub(crate) fn prepare_monthly_slices<'input, 'data: 'input>(
    monthly_input: &'input MonthlyInput<'data>,
    repair_history: Option<crate::RepairHistory>,
    ledger_read: &impl LedgerRead,
    loan_calculation: &impl LoanCalculation,
) -> Result<PreparedMonthlySlices<'input, 'data>, SimulationError> {
    let period = monthly_input.frozen_input.period;
    for instance in monthly_input.activities {
        match &instance.configuration {
            ActivityConfiguration::RepairOccurrence(config) => {
                let history = repair_history.ok_or(SimulationError::InvalidInput(
                    "repair history is required for an active occurrence activity",
                ))?;
                history.age_months(period)?;
                if config.cost_template.delay_months == 0 {
                    return Err(SimulationError::InvalidInput(
                        "repair cost delay must be positive",
                    ));
                }
                let delay = i32::try_from(config.cost_template.delay_months).map_err(|_| {
                    SimulationError::InvalidInput("repair cost delay is out of range")
                })?;
                period.checked_add_months(delay)?;
                if config.cost_template.maximum.is_negative()
                    || config.cost_template.maximum.rounded_for_posting().is_err()
                    || (RepairCostDistribution {
                        maximum: config.cost_template.maximum,
                    })
                    .maximum_minor_units()
                    .is_err()
                {
                    return Err(SimulationError::InvalidInput(
                        "repair cost template maximum is invalid",
                    ));
                }
            }
            ActivityConfiguration::RepairCost(config) => {
                let delay = i32::try_from(config.delay_months).map_err(|_| {
                    SimulationError::InvalidInput("repair cost delay is out of range")
                })?;
                if config.delay_months == 0
                    || config.occurrence_period.checked_add_months(delay)?
                        != config.effective_period
                    || config.effective_period <= config.occurrence_period
                    || config.effective_period != period
                    || config.maximum.is_negative()
                    || config.maximum.rounded_for_posting().is_err()
                    || (RepairCostDistribution {
                        maximum: config.maximum,
                    })
                    .maximum_minor_units()
                    .is_err()
                {
                    return Err(SimulationError::InvalidInput(
                        "repair cost instance configuration is invalid",
                    ));
                }
            }
            _ => {}
        }
    }
    let mut validated = crate::validation::validate_slice_input(
        monthly_input.frozen_input,
        ledger_read,
        monthly_input.opening_state,
    )
    .map_err(SimulationError::InputValidation)?;

    let mut activities = BTreeMap::new();
    for instance in monthly_input.activities {
        if activities.insert(instance.id, instance).is_some() {
            return Err(SimulationError::InvalidInput(
                "duplicate activity instance in monthly input",
            ));
        }
    }

    let mut loans = BTreeMap::new();
    for continuation in monthly_input.loan_continuations {
        if loans
            .insert(continuation.instance_id, continuation)
            .is_some()
        {
            return Err(SimulationError::InvalidInput(
                "duplicate loan continuation in monthly input",
            ));
        }
    }

    let loan_instances = activities
        .values()
        .filter(|instance| matches!(instance.configuration, ActivityConfiguration::Loan(_)))
        .collect::<Vec<_>>();
    if loans.len() != loan_instances.len()
        || loan_instances
            .iter()
            .any(|instance| !loans.contains_key(&instance.id))
    {
        return Err(SimulationError::InvalidInput(
            "monthly input requires one loan continuation for each loan activity",
        ));
    }

    let mut liability_accounts = BTreeSet::new();
    for instance in &loan_instances {
        let ActivityConfiguration::Loan(config) = &instance.configuration else {
            unreachable!("loan instance collection contains only loan configurations")
        };
        let continuation = loans[&instance.id];
        let slice = loan_slice(instance, *config);
        if !liability_accounts.insert(config.liability_account) {
            return Err(SimulationError::InvalidInput(
                "each loan activity must use its own liability account",
            ));
        }
        if instance.slice_id != continuation.slice_id
            || continuation.loan.is_some_and(|loan| {
                loan.opening_balance.currency() != ledger_read.currency()
                    || loan
                        .scheduled_payment
                        .is_some_and(|payment| payment.currency() != ledger_read.currency())
            })
        {
            return Err(SimulationError::InvalidInput(
                "loan continuation identity or currency does not match its activity",
            ));
        }
        validate_loan_opening_state(&slice, continuation.loan, ledger_read)?;
    }
    // Reject malformed activity configurations before invoking injected loan
    // calculations. Model-derived cash-flow bounds are added to the final
    // pre-sampling check below once those calculations have completed.
    validated.validate_activity_configurations(monthly_input.activities, ledger_read)?;
    let mut loan_outputs = BTreeMap::new();
    for instance in &loan_instances {
        if let Some(loan) = loans[&instance.id].loan {
            let ActivityConfiguration::Loan(config) = &instance.configuration else {
                unreachable!("loan instance collection contains only loan configurations")
            };
            let calculation = validate_loan_calculation(loan, loan_calculation)?;
            let output = loan_slice(instance, *config).propose_month(
                monthly_input.frozen_input.period,
                loan,
                calculation,
            )?;
            loan_outputs.insert(instance.id, output);
        }
    }
    let loan_cash_flow_support = loan_outputs.values().flat_map(|output| {
        [
            (CashFlowCategory::Operating, output.result.interest),
            (CashFlowCategory::Financing, output.result.principal),
        ]
    });
    validated.validate_activities_before_sampling(
        monthly_input.activities,
        ledger_read,
        loan_cash_flow_support,
    )?;
    Ok(PreparedMonthlySlices {
        monthly_input,
        validated,
        loan_outputs,
    })
}

impl PreparedMonthlySlices<'_, '_> {
    pub(crate) fn loan_outputs(&self) -> &BTreeMap<ActivityInstanceId, LoanMonthOutput> {
        &self.loan_outputs
    }
}

/// Dispatch every non-loan ActivityConfiguration through the same preflighted
/// sample/evaluate/collect path. Typed samples are converted only at this
/// heterogeneous monthly result boundary.
// The two repair stream maps stay explicit so occurrence and cost random input
// cannot share consumption state; the remaining arguments are existing runtime interfaces.
#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_prepared_monthly_slices<R, US>(
    prepared: PreparedMonthlySlices<'_, '_>,
    repair_history: Option<crate::RepairHistory>,
    utility_sampler: &mut US,
    random: &mut R,
    repair_occurrence_random: &mut BTreeMap<crate::SliceRandomInputKey, R>,
    repair_cost_random: &mut BTreeMap<crate::SliceRandomInputKey, R>,
) -> Result<(Vec<SliceOutput>, Vec<ActivitySample<ActivitySampleValue>>), SimulationError>
where
    R: RandomSource,
    US: Sampler<UtilityUsageDistribution>,
{
    let prepared_history = repair_history;
    let period = prepared.monthly_input.frozen_input.period;
    let mut outputs = Vec::new();
    let mut samples = Vec::new();
    let mut output_ids = BTreeSet::new();

    let mut active_instances = prepared.monthly_input.activities.iter().collect::<Vec<_>>();
    active_instances.sort_by_key(|instance| instance.id);
    for instance in active_instances {
        let output = match &instance.configuration {
            ActivityConfiguration::Rental => {
                let slice = crate::RentalSlice {
                    instance: instance.clone(),
                };
                let sample = ActivitySample::new(instance, period, ());
                let output = propose_slice(&slice, &prepared.validated, &sample)?;
                samples.push(ActivitySample {
                    instance_id: instance.id,
                    slice_id: instance.slice_id,
                    period,
                    purpose: None,
                    value: ActivitySampleValue::Rental,
                });
                output
            }
            ActivityConfiguration::RepairOccurrence(_) => {
                let history =
                    prepared_history.expect("repair occurrence was preflighted with history");
                let distribution = RepairOccurrenceDistribution {
                    age_months: history.age_months(period)?,
                };
                let mut sampler = RepairOccurrenceSampler;
                let source = repair_occurrence_random
                    .get_mut(&crate::SliceRandomInputKey {
                        activity_id: instance.id,
                        slice_id: instance.slice_id,
                        period,
                        purpose: "repair.occurrence",
                    })
                    .ok_or(SimulationError::InvalidInput(
                        "repair occurrence random input is missing for an active instance",
                    ))?;
                let value = sampler
                    .sample(&distribution, source)
                    .map_err(|_| sampling_failure(period, instance.id))?;
                let slice = crate::RepairOccurrenceSlice {
                    instance: instance.clone(),
                };
                let sample = ActivitySample::new(instance, period, value)
                    .with_purpose(Some("repair.occurrence".into()));
                let output = propose_slice(&slice, &prepared.validated, &sample)?;
                samples.push(ActivitySample {
                    instance_id: instance.id,
                    slice_id: instance.slice_id,
                    period,
                    purpose: sample.purpose.clone(),
                    value: ActivitySampleValue::RepairOccurrence(value),
                });
                output
            }
            ActivityConfiguration::RepairCost(config) => {
                if config.effective_period != period
                    || config.effective_period <= config.occurrence_period
                {
                    return Err(SimulationError::InvalidInput(
                        "repair cost is active outside its effective month",
                    ));
                }
                let distribution = RepairCostDistribution {
                    maximum: config.maximum,
                };
                let mut sampler = RepairCostSampler;
                let source = repair_cost_random
                    .get_mut(&crate::SliceRandomInputKey {
                        activity_id: instance.id,
                        slice_id: instance.slice_id,
                        period,
                        purpose: "repair.cost",
                    })
                    .ok_or(SimulationError::InvalidInput(
                        "repair cost random input is missing for an active instance",
                    ))?;
                let value = sampler
                    .sample(&distribution, source)
                    .map_err(|_| sampling_failure(period, instance.id))?;
                let slice = crate::RepairCostSlice {
                    instance: instance.clone(),
                };
                let sample = ActivitySample::new(instance, period, value)
                    .with_purpose(Some("repair.cost".into()));
                let output = propose_slice(&slice, &prepared.validated, &sample)?;
                samples.push(ActivitySample {
                    instance_id: instance.id,
                    slice_id: instance.slice_id,
                    period,
                    purpose: sample.purpose.clone(),
                    value: ActivitySampleValue::RepairCost(value),
                });
                output
            }
            ActivityConfiguration::PropertyTax(_) => {
                let slice = crate::PropertyTaxSlice {
                    instance: instance.clone(),
                };
                let sample = ActivitySample::new(instance, period, ());
                let output = propose_slice(&slice, &prepared.validated, &sample)?;
                samples.push(ActivitySample {
                    instance_id: instance.id,
                    slice_id: instance.slice_id,
                    period,
                    purpose: None,
                    value: ActivitySampleValue::PropertyTax,
                });
                output
            }
            ActivityConfiguration::Utility(config) => {
                let slice = crate::UtilitySlice {
                    instance: instance.clone(),
                };
                let distribution = UtilityUsageDistribution {
                    maximum: config.maximum_usage,
                };
                let sample = sample_utility(
                    &prepared.validated,
                    instance,
                    period,
                    &distribution,
                    utility_sampler,
                    random,
                )
                .map_err(|_| sampling_failure(period, instance.id))?;
                let output = propose_slice(&slice, &prepared.validated, &sample)?;
                samples.push(ActivitySample {
                    instance_id: sample.instance_id,
                    slice_id: sample.slice_id,
                    period: sample.period,
                    purpose: sample.purpose.clone(),
                    value: ActivitySampleValue::Utility(sample.value),
                });
                output
            }
            ActivityConfiguration::Loan(_) => continue,
        };

        if output.instance() != instance
            || output.period() != period
            || output.opening_state() != prepared.monthly_input.opening_state
            || output.frozen_input() != prepared.monthly_input.frozen_input
            || !output_ids.insert(instance.id)
        {
            return Err(SimulationError::InvalidProposal(
                "Slice output does not match its monthly input provenance",
            ));
        }
        outputs.push(output);
    }

    Ok((outputs, samples))
}

pub(crate) fn compose_prepared_monthly_slices(
    outputs: Vec<SliceOutput>,
    ids: &dyn OutputIdAllocator,
) -> Result<IdentifiedProposal, SimulationError> {
    let mut resources = Vec::with_capacity(outputs.len());
    let mut proposals = Vec::with_capacity(outputs.len());
    for output in outputs {
        resources.push(output.resources().clone());
        let instance = output.instance().clone();
        let mut identified = identify_proposal(output, ids)?;
        if let ActivityConfiguration::RepairCost(config) = instance.configuration {
            for effect in &mut identified.effects {
                effect.source_event_id = Some(config.source_event_id);
            }
            for transaction in &mut identified.transactions {
                transaction.source_event_id = Some(config.source_event_id);
            }
        }
        proposals.push(identified);
    }
    validate_composition(&resources, &proposals)?;
    merge_proposals(proposals)
}

pub(crate) fn loan_slice(instance: &ActivityInstance, config: LoanConfig) -> LoanSlice {
    LoanSlice {
        instance_id: instance.id,
        slice_id: instance.slice_id,
        liability_account: config.liability_account,
        interest_expense_account: config.interest_expense_account,
        cash_account: config.cash_account,
    }
}

fn sampling_failure(
    period: property_simulation_domain::YearMonth,
    instance_id: ActivityInstanceId,
) -> SimulationError {
    SimulationError::Fatal(FatalSimulationFailure::new(
        FatalSimulationPhase::SliceTransition,
        period,
        Some(instance_id),
        SimulationError::SamplingFailure,
    ))
}
