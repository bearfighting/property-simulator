use property_simulation_context::Context;
use property_simulation_snapshot::Snapshot;

use crate::{SliceEvaluationError, SliceInstance, SliceResult, slices::SliceInstanceCreator};

/// Property tax behavior instance.
#[derive(Debug, Default)]
pub(crate) struct PropertyTaxSlice;

impl SliceInstance for PropertyTaxSlice {
    fn evaluate(&self) -> Result<SliceResult, SliceEvaluationError> {
        Ok(SliceResult)
    }
}

impl SliceInstanceCreator for PropertyTaxSlice {
    fn create_instances(_context: &Context, _snapshot: &Snapshot) -> Vec<Box<dyn SliceInstance>> {
        Vec::new()
    }
}
