use std::ops::Deref;

use crate::composition::SliceResources;
use crate::error::{FatalSimulationFailure, FatalSimulationPhase, SimulationError};
use crate::types::{
    ActivityInstance, ActivitySampleKind, FrozenInput, OpeningState, RepairSample, SliceProposal,
};
use crate::validation::ValidatedInput;
use property_simulation_domain::{ActivityInstanceId, SliceId, YearMonth};
use property_simulation_models::{
    Distribution, RandomSource, RepairDistribution, Sampler, SamplingError,
    UtilityUsageDistribution,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivitySampleValue {
    Rental,
    Repair(RepairSample),
    Utility(u64),
    Loan(Option<property_simulation_models::FixedRateLoan>),
    PropertyTax,
}

/// An opaque sample token created only by `propose_slice` after provenance checks.
pub struct ValidatedActivitySample<'a, T> {
    value: &'a T,
}
impl<T> ValidatedActivitySample<'_, T> {
    pub fn value(&self) -> &T {
        self.value
    }
}

/// A deliberately small, statically registered Slice contract. Each instance
/// reads only its shared opening state, frozen inputs, and explicit sample;
/// implementations cannot access another instance's result or a random
/// generator. Outputs must obey the associative/commutative proposal merge
/// contract and be compatible with the active composition. Call `propose_slice`
/// to validate provenance before dispatching to a Slice.
pub trait SimulationSlice {
    type Sample;
    const SAMPLE_KIND: ActivitySampleKind;
    fn instance(&self) -> &ActivityInstance;
    /// Declare the resources read from the shared opening context and written
    /// by this active instance, including accounts affected by frozen inputs.
    /// Declarations do not establish dependencies or execution order.
    fn resources(&self, input: &FrozenInput) -> SliceResources;
    /// Preserve the exact typed value used for this evaluation in the result record.
    fn sample_value(&self, sample: &Self::Sample) -> ActivitySampleValue;
    /// Validate the typed sample at the input boundary, before transition logic runs.
    fn validate_sample(
        &self,
        _input: &ValidatedInput,
        _sample: &Self::Sample,
    ) -> Result<(), SimulationError> {
        Ok(())
    }
    #[doc(hidden)]
    fn propose_validated(
        &self,
        state: &OpeningState,
        input: &FrozenInput,
        sample: &ValidatedActivitySample<'_, Self::Sample>,
    ) -> Result<SliceProposal, SimulationError>;
}

/// A proposal bound to the activity instance and period that produced it.
/// Its private fields prevent callers from relabeling a Slice's output before
/// IDs and provenance are assigned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceOutput {
    pub(crate) proposal: SliceProposal,
    pub(crate) instance: ActivityInstance,
    pub(crate) period: YearMonth,
    pub(crate) opening_state: OpeningState,
    pub(crate) frozen_input: FrozenInput,
    pub(crate) resources: SliceResources,
    pub(crate) sample: ActivitySample<ActivitySampleValue>,
}
impl Deref for SliceOutput {
    type Target = SliceProposal;

    fn deref(&self) -> &Self::Target {
        &self.proposal
    }
}

/// Validate a typed sample against the concrete activity instance and frozen
/// period before the Slice receives its value.
pub fn propose_slice<S: SimulationSlice>(
    slice: &S,
    input: &ValidatedInput,
    sample: &ActivitySample<S::Sample>,
) -> Result<SliceOutput, SimulationError> {
    let state = input.opening_state();
    let frozen_input = input.frozen_input();
    sample.validate_for(slice.instance(), frozen_input.period)?;
    if slice.instance().configuration.sample_kind() != S::SAMPLE_KIND {
        return Err(SimulationError::InvalidInput(
            "activity sample kind does not match Slice",
        ));
    }
    slice.validate_sample(input, &sample.value)?;
    let resources = slice.resources(frozen_input);
    if resources.instance_id != slice.instance().id
        || resources.slice_id != slice.instance().slice_id
    {
        return Err(SimulationError::InvalidProposal(
            "resource declaration provenance does not match Slice",
        ));
    }
    let proposal = slice
        .propose_validated(
            state,
            frozen_input,
            &ValidatedActivitySample {
                value: &sample.value,
            },
        )
        .map_err(|source| {
            SimulationError::Fatal(FatalSimulationFailure::new(
                FatalSimulationPhase::SliceTransition,
                frozen_input.period,
                Some(slice.instance().id),
                source,
            ))
        })?;
    Ok(SliceOutput {
        proposal,
        instance: slice.instance().clone(),
        period: frozen_input.period,
        opening_state: state.clone(),
        frozen_input: *frozen_input,
        resources,
        sample: ActivitySample::new(
            slice.instance(),
            frozen_input.period,
            slice.sample_value(&sample.value),
        )
        .with_purpose(sample.purpose.clone()),
    })
}

/// Typed samples retain their source and effective period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivitySample<T> {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub period: YearMonth,
    /// Semantic random purpose, absent for caller supplied deterministic samples.
    pub purpose: Option<String>,
    pub value: T,
}

pub const REPAIR_SAMPLE_PURPOSE: &str = "repair.sample";
pub const UTILITY_SAMPLE_PURPOSE: &str = "utility.usage";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtilitySamplingError<E> {
    InvalidDistribution,
    ActivitySetNotPreflighted,
    PeriodMismatch,
    Sampler(SamplingError<E>),
}

/// Sample utility usage from caller-supplied randomness.
/// The supplied input must have preflighted the complete active activity set
/// for this month before any random sampler is called.
pub fn sample_utility<R, S>(
    validated: &ValidatedInput,
    instance: &ActivityInstance,
    period: YearMonth,
    distribution: &UtilityUsageDistribution,
    sampler: &mut S,
    random: &mut R,
) -> Result<ActivitySample<u64>, UtilitySamplingError<R::Error>>
where
    R: RandomSource,
    S: Sampler<UtilityUsageDistribution>,
{
    if !matches!(
        &instance.configuration,
        crate::types::ActivityConfiguration::Utility(config)
            if distribution.maximum == config.maximum_usage
    ) {
        return Err(UtilitySamplingError::InvalidDistribution);
    }
    if !validated.has_preflighted_activity(instance) {
        return Err(UtilitySamplingError::ActivitySetNotPreflighted);
    }
    if validated.frozen_input().period != period {
        return Err(UtilitySamplingError::PeriodMismatch);
    }
    let value = sampler
        .sample(distribution, random)
        .map_err(UtilitySamplingError::Sampler)?;
    Ok(ActivitySample::new(instance, period, value)
        .with_purpose(Some(UTILITY_SAMPLE_PURPOSE.into())))
}

/// Composition boundary for the repair model: validates distribution/Slice
/// compatibility, samples from the injected source, and binds provenance.
pub fn sample_repair<R, S>(
    instance: &ActivityInstance,
    period: YearMonth,
    distribution: &RepairDistribution,
    sampler: &mut S,
    random: &mut R,
) -> Result<ActivitySample<RepairSample>, SamplingError<R::Error>>
where
    R: RandomSource,
    S: Sampler<RepairDistribution>,
{
    if instance.configuration.sample_kind() != ActivitySampleKind::Repair
        || !distribution.validate()
    {
        return Err(SamplingError::InvalidDistribution);
    }
    let value = sampler.sample(distribution, random)?;
    Ok(ActivitySample::new(instance, period, value)
        .with_purpose(Some(REPAIR_SAMPLE_PURPOSE.into())))
}
impl<T> ActivitySample<T> {
    pub fn new(instance: &ActivityInstance, period: YearMonth, value: T) -> Self {
        Self {
            instance_id: instance.id,
            slice_id: instance.slice_id,
            period,
            purpose: None,
            value,
        }
    }

    pub fn with_purpose(mut self, purpose: Option<String>) -> Self {
        self.purpose = purpose;
        self
    }

    pub fn validate_for(
        &self,
        instance: &ActivityInstance,
        period: YearMonth,
    ) -> Result<(), SimulationError> {
        if self.instance_id != instance.id
            || self.slice_id != instance.slice_id
            || self.period != period
        {
            return Err(SimulationError::InvalidInput(
                "sample provenance does not match activity instance or period",
            ));
        }
        if instance.configuration.sample_kind() == ActivitySampleKind::Repair
            && self
                .purpose
                .as_deref()
                .is_some_and(|purpose| purpose != REPAIR_SAMPLE_PURPOSE)
        {
            return Err(SimulationError::InvalidInput(
                "repair sample purpose does not match the repair sampling contract",
            ));
        }
        if instance.configuration.sample_kind() == ActivitySampleKind::Utility
            && self
                .purpose
                .as_deref()
                .is_some_and(|purpose| purpose != UTILITY_SAMPLE_PURPOSE)
        {
            return Err(SimulationError::InvalidInput(
                "utility sample purpose does not match the utility sampling contract",
            ));
        }
        Ok(())
    }
}
