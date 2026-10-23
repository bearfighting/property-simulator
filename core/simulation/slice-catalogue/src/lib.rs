//! Types describing the Slice kinds available to Application.

mod slice_kind;
mod slice_parameters;
mod slice_setting;
mod slice_settings;

pub use slice_kind::SliceKind;
pub use slice_parameters::{
    LoanParameters, LoanParametersError, PropertyTaxParameters, RentalParameters,
    RepairCostParameters, RepairOccurrenceParameters, UtilityParameters,
};
pub use slice_setting::SliceSetting;
pub use slice_settings::SliceSettings;
