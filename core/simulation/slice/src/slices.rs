use crate::SliceInstance;
use property_simulation_context::Context;
use property_simulation_snapshot::Snapshot;

mod loan;
mod property_tax;
mod rental;
mod repair_cost;
mod repair_occurrence;
mod utility;

pub(crate) use loan::LoanSlice;
pub(crate) use property_tax::PropertyTaxSlice;
pub(crate) use rental::RentalSlice;
pub(crate) use repair_cost::RepairCostSlice;
pub(crate) use repair_occurrence::RepairOccurrenceSlice;
pub(crate) use utility::UtilitySlice;

/// Creates zero or more runtime instances for one Slice kind.
///
/// Implementations own the kind-specific rules for reading Context/Snapshot
/// and constructing its instances.
pub(crate) trait SliceInstanceCreator {
    fn create_instances(context: &Context, snapshot: &Snapshot) -> Vec<Box<dyn SliceInstance>>;
}
