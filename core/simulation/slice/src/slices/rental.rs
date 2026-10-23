use property_simulation_context::Context;
use property_simulation_snapshot::Snapshot;

use crate::{SliceEvaluationError, SliceInstance, SliceResult, slices::SliceInstanceCreator};

/// Rental behavior instance.
#[derive(Debug, Default)]
pub(crate) struct RentalSlice;

impl SliceInstance for RentalSlice {
    fn evaluate(&self) -> Result<SliceResult, SliceEvaluationError> {
        Ok(SliceResult)
    }
}

impl SliceInstanceCreator for RentalSlice {
    fn create_instances(_context: &Context, _snapshot: &Snapshot) -> Vec<Box<dyn SliceInstance>> {
        Vec::new()
    }
}
