use crate::{
    PropertyTaxParameters, RentalParameters, RepairCostParameters, RepairOccurrenceParameters,
    SliceKind, UtilityParameters,
};

/// User settings for one supported Slice kind.
///
/// The enum variant identifies the kind and binds it to its parameters where
/// those parameters are per-Slice settings. Initial loan terms are part of
/// `InitialValues`. Only directly user-selectable kinds carry an `enabled`
/// value. `RepairCost` settings have no independent enable flag because
/// instances are driven by repair occurrence and their own lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SliceSetting {
    /// Enables or disables the loan behavior; initial loan terms are held in
    /// `InitialValues` because they cannot be changed during the simulation.
    Loan {
        enabled: bool,
    },
    PropertyTax {
        enabled: bool,
        parameters: PropertyTaxParameters,
    },
    Rental {
        enabled: bool,
        parameters: RentalParameters,
    },
    RepairOccurrence {
        enabled: bool,
        parameters: RepairOccurrenceParameters,
    },
    RepairCost {
        parameters: RepairCostParameters,
    },
    Utility {
        enabled: bool,
        parameters: UtilityParameters,
    },
}

impl SliceSetting {
    /// Return the kind encoded by this setting variant.
    pub const fn kind(&self) -> SliceKind {
        match self {
            Self::Loan { .. } => SliceKind::Loan,
            Self::PropertyTax { .. } => SliceKind::PropertyTax,
            Self::Rental { .. } => SliceKind::Rental,
            Self::RepairOccurrence { .. } => SliceKind::RepairOccurrence,
            Self::RepairCost { .. } => SliceKind::RepairCost,
            Self::Utility { .. } => SliceKind::Utility,
        }
    }
}
