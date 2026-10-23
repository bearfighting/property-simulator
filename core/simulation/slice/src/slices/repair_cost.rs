use property_simulation_context::Context;
use property_simulation_snapshot::Snapshot;

use crate::{SliceEvaluationError, SliceInstance, SliceResult, slices::SliceInstanceCreator};

/// One-time repair cost behavior instance.
#[derive(Debug, Default)]
pub(crate) struct RepairCostSlice;

impl SliceInstance for RepairCostSlice {
    fn evaluate(&self) -> Result<SliceResult, SliceEvaluationError> {
        Ok(SliceResult)
    }
}

impl SliceInstanceCreator for RepairCostSlice {
    fn create_instances(_context: &Context, _snapshot: &Snapshot) -> Vec<Box<dyn SliceInstance>> {
        Vec::new()
    }
}
