/// Settings required to determine recurring repair occurrences.
///
/// **Incomplete:** this placeholder has no occurrence or repair-cost template
/// fields. Consumers must not assume it can configure repair occurrences.
/// Complete it when occurrence inputs and their relationship to generated
/// repair-cost settings are agreed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepairOccurrenceParameters;
