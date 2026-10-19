/// Classification attached to a cash-flow component or posting purpose.
/// It belongs to the transaction purpose, not to an account definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CashFlowCategory {
    Operating,
    Investing,
    Financing,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provides_the_three_direct_cash_flow_categories() {
        let categories = [
            CashFlowCategory::Operating,
            CashFlowCategory::Investing,
            CashFlowCategory::Financing,
        ];
        assert_eq!(categories.len(), 3);
        assert_ne!(categories[0], categories[2]);
    }
}
