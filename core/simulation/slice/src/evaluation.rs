/// Common result returned by evaluating any Slice instance.
///
/// **Incomplete:** this placeholder carries no events, JournalEntry values, or
/// continuation effects. Complete it when at least two representative Slices
/// can express their full monthly output through this type without losing
/// accounting balance or required continuation/audit facts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SliceResult;

/// Common failure type for Slice instance evaluation.
///
/// **Incomplete:** this placeholder does not distinguish input validation,
/// domain calculation, or JournalEntry construction failures. Complete it when
/// representative Slices define their expected failure cases and the caller
/// can distinguish recoverable input errors from fatal calculation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceEvaluationError;

/// One runtime Slice instance with a common evaluation operation.
///
/// **Incomplete:** the current method has no explicit input parameter and
/// assumes instances capture all required state. Complete this trait when two
/// representative Slice implementations demonstrate that the shared call
/// contract can supply their month, opening facts, configuration, and random
/// input while returning the agreed common result and error.
pub trait SliceInstance {
    /// **Incomplete:** evaluates without an explicit input contract. Complete
    /// this method with the shared per-month input once representative Slice
    /// implementations settle which values are supplied and which are captured
    /// when the instance is created.
    fn evaluate(&self) -> Result<SliceResult, SliceEvaluationError>;
}
