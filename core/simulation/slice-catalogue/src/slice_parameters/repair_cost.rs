/// Settings required to calculate a repair cost instance.
///
/// **Incomplete:** this placeholder has no amount, distribution, or delay
/// fields. Consumers must not assume it can configure repair costs. Complete
/// it when a generated repair-cost instance's required inputs and delay rules
/// are agreed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepairCostParameters;
