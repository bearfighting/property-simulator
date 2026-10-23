use property_simulation_context::Context;
use property_simulation_snapshot::Snapshot;

use crate::{
    SliceInstance,
    slices::{
        LoanSlice, PropertyTaxSlice, RentalSlice, RepairCostSlice, RepairOccurrenceSlice,
        SliceInstanceCreator, UtilitySlice,
    },
};

/// **Incomplete:** dispatches every current Slice kind using placeholder
/// Context/Snapshot values and has no creation error path. Complete this
/// function when enabled Slice configurations and current-month inputs are
/// represented, and invalid configuration can be reported without silently
/// omitting required instances.
///
/// Each Slice kind decides whether it has instances to create and may return
/// zero or more. The returned vector is a collection only; its order does not
/// define evaluation order or dependencies.
pub fn create_slices(context: &Context, snapshot: &Snapshot) -> Vec<Box<dyn SliceInstance>> {
    let mut instances = Vec::new();

    instances.extend(RentalSlice::create_instances(context, snapshot));
    instances.extend(RepairOccurrenceSlice::create_instances(context, snapshot));
    instances.extend(RepairCostSlice::create_instances(context, snapshot));
    instances.extend(LoanSlice::create_instances(context, snapshot));
    instances.extend(PropertyTaxSlice::create_instances(context, snapshot));
    instances.extend(UtilitySlice::create_instances(context, snapshot));

    instances
}
