use property_simulation_month::Month;

/// A state change produced by a completed Slice evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Remaining scheduled loan payments after the current month's payment.
    /// Zero means the loan has been paid off.
    LoanRemainingTermUpdated { remaining_months: u32 },
    /// Month of the latest settled repair cost, used as the next occurrence baseline.
    RepairCostBaselineUpdated { month: Month },
}
