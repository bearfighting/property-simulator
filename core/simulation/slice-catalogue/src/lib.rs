//! Per-Slice parameter, input, and single-invocation output contracts.
//!
//! Application selects and invokes concrete Slices. This crate defines their
//! shared value contracts but does not coordinate or aggregate Slice calls.

mod event;
mod slice_inputs;
mod slice_kind;
mod slice_parameters;
mod slice_result;
mod slice_result_error;

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
