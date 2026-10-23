#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomnessError {
    EntropyUnavailable,
    TrialIdentityMismatch,
}
