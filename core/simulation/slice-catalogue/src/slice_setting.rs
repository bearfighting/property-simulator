use crate::{FixedExpenseParameters, RentalParameters, RepairCostParameters, SliceKind};

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
    FixedExpense {
        enabled: bool,
        parameters: FixedExpenseParameters,
    },
    Rental {
        enabled: bool,
        parameters: RentalParameters,
    },
    RepairOccurrence {
        enabled: bool,
    },
    RepairCost {
        parameters: RepairCostParameters,
    },
}

impl SliceSetting {
    /// Return the kind encoded by this setting variant.
    pub const fn kind(&self) -> SliceKind {
        match self {
            Self::Loan { .. } => SliceKind::Loan,
            Self::FixedExpense { .. } => SliceKind::FixedExpense,
            Self::Rental { .. } => SliceKind::Rental,
            Self::RepairOccurrence { .. } => SliceKind::RepairOccurrence,
            Self::RepairCost { .. } => SliceKind::RepairCost,
        }
    }
}
