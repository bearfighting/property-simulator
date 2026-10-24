//! Closed contracts for the Slice kinds available to Application.
//!
//! This crate defines user parameters, per-period inputs, and typed outputs.
//! It does not depend on Context, Snapshot, or Slice implementations.

mod effect;
mod event;
mod slice_inputs;
mod slice_kind;
mod slice_parameters;
mod slice_result;
mod slice_result_error;
mod slice_setting;
mod slice_settings;

pub use effect::Effect;
pub use event::Event;
pub use slice_inputs::{
    FixedExpenseSliceInput, LoanSliceInput, RentalSliceInput, RepairCostSliceInput,
    RepairOccurrenceSliceInput,
};
pub use slice_kind::SliceKind;
pub use slice_parameters::{
    FixedExpenseMethod, FixedExpenseParameters, FixedExpenseParametersError, LoanParameters,
    LoanParametersError, RentalParameters, RentalParametersError, RepairCostParameters,
    RepairCostParametersError,
};
pub use slice_result::SliceResult;
pub use slice_result_error::SliceResultError;
pub use slice_setting::SliceSetting;
pub use slice_settings::SliceSettings;
