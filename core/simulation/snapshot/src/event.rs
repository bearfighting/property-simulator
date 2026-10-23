/// A domain event recorded in a completed-month snapshot.
///
/// **Incomplete:** variants currently carry no data. Add their payloads when
/// representative Slice outcomes establish which facts must persist for audit
/// and continuation. Complete this enum when every implemented event retains
/// its period, relevant business values, and any required source relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    RentCollected,
    RepairOccurred,
    RepairCostSettled,
    LoanPayment,
    PropertyTaxPayment,
    UtilityUsageCharged,
}
