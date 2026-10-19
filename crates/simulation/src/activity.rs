use std::ops::Deref;

use property_simulation_domain::{ActivityInstanceId, ModelVersionId, SliceId, YearMonth};

use crate::error::SimulationError;
use crate::types::{
    ActivityInstance, ActivitySampleKind, FrozenInput, OpeningState, RepairSample, SliceProposal,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivitySampleValue {
    Rental,
    Repair(RepairSample),
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

/// A deliberately small, statically registered Slice contract. Samples are
/// explicit inputs; implementations cannot access a random generator. Call
/// `propose_slice` to validate provenance before dispatching to a Slice.
pub trait SimulationSlice {
    type Sample;
    const SAMPLE_KIND: ActivitySampleKind;
    fn instance(&self) -> &ActivityInstance;
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
    state: &OpeningState,
    input: &FrozenInput,
    sample: &ActivitySample<S::Sample>,
) -> Result<SliceOutput, SimulationError> {
    sample.validate_for(slice.instance(), input.period)?;
    if slice.instance().sample_kind != S::SAMPLE_KIND {
        return Err(SimulationError::InvalidInput(
            "activity sample kind does not match Slice",
        ));
    }
    let proposal = slice.propose_validated(
        state,
        input,
        &ValidatedActivitySample {
            value: &sample.value,
        },
    )?;
    Ok(SliceOutput {
        proposal,
        instance: slice.instance().clone(),
        period: input.period,
    })
}

/// Typed samples retain their source and effective period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivitySample<T> {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub version: ModelVersionId,
    pub period: YearMonth,
    pub value: T,
}
impl<T> ActivitySample<T> {
    pub fn new(instance: &ActivityInstance, period: YearMonth, value: T) -> Self {
        Self {
            instance_id: instance.id,
            slice_id: instance.slice_id,
            version: instance.version,
            period,
            value,
        }
    }

    pub fn validate_for(
        &self,
        instance: &ActivityInstance,
        period: YearMonth,
    ) -> Result<(), SimulationError> {
        if self.instance_id != instance.id
            || self.slice_id != instance.slice_id
            || self.version != instance.version
            || self.period != period
        {
            return Err(SimulationError::InvalidInput(
                "sample provenance does not match activity instance or period",
            ));
        }
        Ok(())
    }
}
