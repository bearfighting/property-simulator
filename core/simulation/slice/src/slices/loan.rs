use property_simulation_context::Context;
use property_simulation_snapshot::Snapshot;

use crate::{SliceEvaluationError, SliceInstance, SliceResult, slices::SliceInstanceCreator};

/// Loan behavior instance.
#[derive(Debug, Default)]
pub(crate) struct LoanSlice;

impl SliceInstance for LoanSlice {
    fn evaluate(&self) -> Result<SliceResult, SliceEvaluationError> {
        Ok(SliceResult)
    }
}

impl SliceInstanceCreator for LoanSlice {
    fn create_instances(_context: &Context, _snapshot: &Snapshot) -> Vec<Box<dyn SliceInstance>> {
        Vec::new()
    }
}
