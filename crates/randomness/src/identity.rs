use property_simulation_domain::{ActivityInstanceId, SliceId, TrialId, YearMonth};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RandomCategory {
    Environment = 1,
    Property = 2,
    PolicyMacro = 3,
    OccupancyOperations = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticIdentity {
    pub trial_id: TrialId,
    pub category: RandomCategory,
    pub slice_id: SliceId,
    pub activity_id: ActivityInstanceId,
    pub period: YearMonth,
    pub purpose: String,
}
