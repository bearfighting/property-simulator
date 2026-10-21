use std::collections::{BTreeMap, BTreeSet};

use crate::activity::{
    ActivitySample, ActivitySampleValue, SimulationSlice, SliceOutput, propose_slice,
};
use crate::composition::validate_composition;
use crate::error::SimulationError;
use crate::proposals::{identify_proposal, merge_proposals};
use crate::types::{
    ActivityInstance, ActivitySampleKind, IdentifiedProposal, MonthlyInput, OutputIdFactory,
};
use crate::validation::{LedgerRead, RepairCostValidation, ValidatedInput, validate_input};

/// A concrete Slice bound to the explicitly supplied sample for one activity.
/// The associated sample type stays statically checked by [`SimulationSlice`];
/// this small object-safe boundary lets a month call heterogeneous Slice types.
pub trait MonthlySliceAdapter {
    fn instance(&self) -> &ActivityInstance;
    fn sample_kind(&self) -> ActivitySampleKind;
    fn sample_record(
        &self,
        period: property_simulation_domain::YearMonth,
    ) -> Result<ActivitySample<ActivitySampleValue>, SimulationError>;
    fn evaluate(&self, input: &ValidatedInput) -> Result<SliceOutput, SimulationError>;
}

/// Binds one concrete Slice implementation to its typed sample.
pub struct BoundSliceAdapter<'a, S: SimulationSlice> {
    slice: &'a S,
    sample: &'a ActivitySample<S::Sample>,
}

impl<'a, S: SimulationSlice> BoundSliceAdapter<'a, S> {
    pub const fn new(slice: &'a S, sample: &'a ActivitySample<S::Sample>) -> Self {
        Self { slice, sample }
    }
}

impl<S: SimulationSlice> MonthlySliceAdapter for BoundSliceAdapter<'_, S> {
    fn instance(&self) -> &ActivityInstance {
        self.slice.instance()
    }

    fn sample_kind(&self) -> ActivitySampleKind {
        S::SAMPLE_KIND
    }

    fn sample_record(
        &self,
        period: property_simulation_domain::YearMonth,
    ) -> Result<ActivitySample<ActivitySampleValue>, SimulationError> {
        self.sample.validate_for(self.slice.instance(), period)?;
        Ok(ActivitySample {
            instance_id: self.sample.instance_id,
            slice_id: self.sample.slice_id,
            period: self.sample.period,
            purpose: self.sample.purpose.clone(),
            value: self.slice.sample_value(&self.sample.value),
        })
    }

    fn evaluate(&self, input: &ValidatedInput) -> Result<SliceOutput, SimulationError> {
        propose_slice(self.slice, input, self.sample)
    }
}

/// Validate the opening context and evaluate each ordinary Slice independently.
/// `ledger_read` must read from `monthly_input.opening_ledger`; this relationship
/// is a caller contract because `LedgerRead` intentionally does not expose a
/// concrete Ledger identity. Loan activities are kept for the loan-specific M6
/// adapter boundary and are not dispatched through `SimulationSlice` here.
pub fn evaluate_monthly_slices(
    monthly_input: &MonthlyInput<'_>,
    ledger_read: &impl LedgerRead,
    repair_validation: &dyn RepairCostValidation,
    adapters: &[&dyn MonthlySliceAdapter],
) -> Result<Vec<SliceOutput>, SimulationError> {
    let prepared = prepare_monthly_slices(monthly_input, ledger_read, repair_validation, adapters)?;
    evaluate_prepared_monthly_slices(prepared)
}

/// A fully preflighted ordinary-Slice call, ready for evaluation once monthly
/// loan validation has also succeeded.
pub(crate) struct PreparedMonthlySlices<'input, 'data: 'input, 'adapter> {
    monthly_input: &'input MonthlyInput<'data>,
    validated: ValidatedInput,
    adapters: Vec<(
        property_simulation_domain::ActivityInstanceId,
        &'adapter dyn MonthlySliceAdapter,
    )>,
}

/// Validate every correctable monthly input and adapter binding without
/// invoking either loan mathematics or Slice transition logic.
pub(crate) fn prepare_monthly_slices<'input, 'data: 'input, 'adapter>(
    monthly_input: &'input MonthlyInput<'data>,
    ledger_read: &impl LedgerRead,
    repair_validation: &dyn RepairCostValidation,
    adapters: &'adapter [&'adapter dyn MonthlySliceAdapter],
) -> Result<PreparedMonthlySlices<'input, 'data, 'adapter>, SimulationError> {
    let mut validated = validate_input(
        monthly_input.frozen_input,
        monthly_input.repair_distribution,
        repair_validation,
        ledger_read,
        monthly_input.opening_state,
    )
    .map_err(SimulationError::InputValidation)?;

    let mut activities_by_id = BTreeMap::new();
    for activity in monthly_input.activities {
        if activities_by_id.insert(activity.id, activity).is_some() {
            return Err(SimulationError::InvalidInput(
                "duplicate activity instance in monthly input",
            ));
        }
    }
    let loan_activities = monthly_input
        .activities
        .iter()
        .filter(|activity| activity.configuration.sample_kind() == ActivitySampleKind::Loan)
        .map(|activity| (activity.id, activity))
        .collect::<BTreeMap<_, _>>();
    let mut loans_by_id = BTreeMap::new();
    for continuation in monthly_input.loan_continuations {
        if loans_by_id
            .insert(continuation.instance_id, continuation)
            .is_some()
        {
            return Err(SimulationError::InvalidInput(
                "duplicate loan continuation in monthly input",
            ));
        }
        if loan_activities
            .get(&continuation.instance_id)
            .is_none_or(|activity| activity.slice_id != continuation.slice_id)
        {
            return Err(SimulationError::InvalidInput(
                "loan continuation does not match an active loan activity",
            ));
        }
        if continuation.loan.is_some_and(|loan| {
            loan.opening_balance.currency() != ledger_read.currency()
                || loan
                    .scheduled_payment
                    .is_some_and(|payment| payment.currency() != ledger_read.currency())
        }) {
            return Err(SimulationError::InvalidInput(
                "loan continuation amounts must use the Ledger currency",
            ));
        }
    }
    if loans_by_id.len() != loan_activities.len()
        || loan_activities
            .keys()
            .any(|instance_id| !loans_by_id.contains_key(instance_id))
    {
        return Err(SimulationError::InvalidInput(
            "monthly input requires one loan continuation for each loan activity",
        ));
    }
    let slice_activities = monthly_input
        .activities
        .iter()
        .filter(|activity| activity.configuration.sample_kind() != ActivitySampleKind::Loan)
        .collect::<Vec<_>>();
    validated.validate_activities_before_sampling(monthly_input.activities, ledger_read, [])?;

    let mut samples_by_id = BTreeMap::new();
    for sample in monthly_input.samples {
        if samples_by_id.insert(sample.instance_id, sample).is_some() {
            return Err(SimulationError::InvalidInput(
                "duplicate sample in monthly input",
            ));
        }
    }
    if samples_by_id.len() != slice_activities.len() {
        return Err(SimulationError::InvalidInput(
            "monthly input requires one sample for each non-loan activity",
        ));
    }

    let mut adapters_by_id = BTreeMap::new();
    for adapter in adapters {
        let instance = adapter.instance();
        if instance.configuration.sample_kind() == ActivitySampleKind::Loan
            || adapter.sample_kind() != instance.configuration.sample_kind()
            || activities_by_id
                .get(&instance.id)
                .is_none_or(|expected| *expected != instance)
        {
            return Err(SimulationError::InvalidInput(
                "Slice adapter does not match an active non-loan activity",
            ));
        }
        if adapters_by_id.insert(instance.id, *adapter).is_some() {
            return Err(SimulationError::InvalidInput(
                "duplicate Slice adapter for monthly activity",
            ));
        }
        let Some(expected_sample) = samples_by_id.get(&instance.id) else {
            return Err(SimulationError::InvalidInput(
                "Slice adapter has no matching monthly sample",
            ));
        };
        let sample_record = adapter.sample_record(monthly_input.frozen_input.period)?;
        let sample_kind = match &sample_record.value {
            ActivitySampleValue::Rental => ActivitySampleKind::Rental,
            ActivitySampleValue::Repair(_) => ActivitySampleKind::Repair,
            ActivitySampleValue::Utility(_) => ActivitySampleKind::Utility,
            ActivitySampleValue::PropertyTax => ActivitySampleKind::PropertyTax,
            ActivitySampleValue::Loan(_) => ActivitySampleKind::Loan,
        };
        if sample_kind != instance.configuration.sample_kind() || sample_record != **expected_sample
        {
            return Err(SimulationError::InvalidInput(
                "typed Slice sample kind or value does not match monthly input sample",
            ));
        }
    }
    if adapters_by_id.len() != slice_activities.len()
        || slice_activities
            .iter()
            .any(|activity| !adapters_by_id.contains_key(&activity.id))
    {
        return Err(SimulationError::InvalidInput(
            "monthly input requires one Slice adapter for each non-loan activity",
        ));
    }
    if samples_by_id
        .keys()
        .any(|id| !adapters_by_id.contains_key(id))
    {
        return Err(SimulationError::InvalidInput(
            "monthly input contains a sample without a Slice adapter",
        ));
    }

    Ok(PreparedMonthlySlices {
        monthly_input,
        validated,
        adapters: adapters_by_id.into_iter().collect(),
    })
}

pub(crate) fn evaluate_prepared_monthly_slices(
    prepared: PreparedMonthlySlices<'_, '_, '_>,
) -> Result<Vec<SliceOutput>, SimulationError> {
    let monthly_input = prepared.monthly_input;
    let activities_by_id = monthly_input
        .activities
        .iter()
        .map(|activity| (activity.id, activity))
        .collect::<BTreeMap<_, _>>();
    let samples_by_id = monthly_input
        .samples
        .iter()
        .map(|sample| (sample.instance_id, sample))
        .collect::<BTreeMap<_, _>>();
    let mut output = Vec::with_capacity(prepared.adapters.len());
    let mut output_ids = BTreeSet::new();
    for (instance_id, adapter) in prepared.adapters {
        let slice_output = adapter.evaluate(&prepared.validated)?;
        let Some(expected_instance) = activities_by_id.get(&instance_id) else {
            return Err(SimulationError::InvalidInput(
                "prepared Slice adapter no longer matches monthly input",
            ));
        };
        let Some(expected_sample) = samples_by_id.get(&instance_id) else {
            return Err(SimulationError::InvalidInput(
                "prepared Slice sample no longer matches monthly input",
            ));
        };
        if slice_output.instance() != *expected_instance
            || slice_output.sample() != *expected_sample
            || slice_output.period() != monthly_input.frozen_input.period
            || slice_output.opening_state() != monthly_input.opening_state
            || slice_output.frozen_input() != monthly_input.frozen_input
            || !output_ids.insert(slice_output.instance().id)
        {
            return Err(SimulationError::InvalidProposal(
                "Slice output does not match its monthly input provenance",
            ));
        }
        output.push(slice_output);
    }
    Ok(output)
}

/// Evaluate and combine all ordinary Slice outputs for one month. Every Slice
/// sees the same validated opening context; declarations and identified
/// proposal provenance are checked before compatible proposals are merged.
/// Loan calculations remain outside this Slice-only composition boundary.
pub fn compose_monthly_slices(
    monthly_input: &MonthlyInput<'_>,
    ledger_read: &impl LedgerRead,
    repair_validation: &dyn RepairCostValidation,
    adapters: &[&dyn MonthlySliceAdapter],
    ids: &dyn OutputIdFactory,
) -> Result<IdentifiedProposal, SimulationError> {
    let prepared = prepare_monthly_slices(monthly_input, ledger_read, repair_validation, adapters)?;
    compose_prepared_monthly_slices(prepared, ids)
}

pub(crate) fn compose_prepared_monthly_slices(
    prepared: PreparedMonthlySlices<'_, '_, '_>,
    ids: &dyn OutputIdFactory,
) -> Result<IdentifiedProposal, SimulationError> {
    let outputs = evaluate_prepared_monthly_slices(prepared)?;
    let mut resources = Vec::with_capacity(outputs.len());
    let mut proposals = Vec::with_capacity(outputs.len());
    for output in outputs {
        resources.push(output.resources().clone());
        proposals.push(identify_proposal(output, ids)?);
    }
    validate_composition(&resources, &proposals)?;
    merge_proposals(proposals)
}
