macro_rules! opaque_id {
    ($($name:ident),+ $(,)?) => { $(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 16]);
        impl $name {
            pub const fn from_bytes(bytes: [u8; 16]) -> Self { Self(bytes) }
            pub const fn as_bytes(&self) -> &[u8; 16] { &self.0 }
        }
    )+ };
}

opaque_id!(
    ScenarioId,
    RunId,
    TrialId,
    BranchId,
    SliceId,
    EntityId,
    EventId,
    DecisionId,
    LedgerTransactionId,
    AccountId,
    ActivityInstanceId,
    EffectId,
    SnapshotId
);

/// Stable, extensible key for a shared simulation resource.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceKey(String);

impl ResourceKey {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("resource key cannot be empty");
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_preserve_fixed_width_bytes_and_have_distinct_types() {
        let bytes = [7; 16];
        let scenario_id = ScenarioId::from_bytes(bytes);
        assert_eq!(scenario_id.as_bytes(), &bytes);
        assert_eq!(scenario_id, ScenarioId::from_bytes(bytes));

        let run_id = RunId::from_bytes(bytes);
        assert_eq!(run_id.as_bytes(), &bytes);
        assert_eq!(run_id, RunId::from_bytes(bytes));
    }
}
