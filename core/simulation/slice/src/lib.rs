//! Slice instance creation and evaluation contracts.

mod collection;
mod evaluation;
mod slices;

pub use collection::create_slices;
pub use evaluation::{SliceEvaluationError, SliceInstance, SliceResult};
