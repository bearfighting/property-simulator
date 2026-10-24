//! Independent Slice implementations for one-period calculations.

mod evaluation;
mod slices;

pub use evaluation::{DeterministicSlice, StochasticSlice};
pub use property_simulation_slice_catalogue::SliceResult;
pub use slices::{
    FixedExpenseSlice, FixedExpenseSliceError, LoanSlice, LoanSliceError, RentalSlice,
    RentalSliceError, RepairCostSlice, RepairCostSliceError, RepairOccurrenceSlice,
    RepairOccurrenceSliceError,
};
