/// An event kind produced by a Slice evaluation.
///
/// The containing SliceResult supplies the Month. Monetary values are read
/// from the Ledger projection and are not duplicated in event values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Rent was collected during the month.
    RentCollected,
    /// A repair occurrence was generated for later cost settlement.
    RepairOccurred,
    /// A repair cost instance settled during the month, including a zero amount.
    RepairCostSettled,
    /// A scheduled loan payment was made during the month.
    LoanPayment,
    /// A recurring fixed expense was paid during the month.
    FixedExpensePaid,
}
