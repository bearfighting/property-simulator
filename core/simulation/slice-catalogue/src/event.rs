use property_simulation_money::Money;

/// A domain event produced by an evaluated Slice and retained in Snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    RentCollected {
        amount: Money,
    },
    RepairOccurred,
    RepairCostSettled {
        amount: Money,
    },
    LoanPayment {
        principal: Money,
        interest: Money,
        closing_balance: Money,
    },
    FixedExpensePaid {
        amount: Money,
    },
}
