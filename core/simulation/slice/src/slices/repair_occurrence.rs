use property_simulation_context::Context;
use property_simulation_snapshot::Snapshot;

use crate::{SliceEvaluationError, SliceInstance, SliceResult, slices::SliceInstanceCreator};

/// Recurring repair occurrence behavior instance.
#[derive(Debug, Default)]
pub(crate) struct RepairOccurrenceSlice;

impl SliceInstance for RepairOccurrenceSlice {
    fn evaluate(&self) -> Result<SliceResult, SliceEvaluationError> {
        Ok(SliceResult)
    }
}

impl SliceInstanceCreator for RepairOccurrenceSlice {
    fn create_instances(_context: &Context, _snapshot: &Snapshot) -> Vec<Box<dyn SliceInstance>> {
        Vec::new()
    }
}
