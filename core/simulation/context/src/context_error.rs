/// Failure to apply a new latest-settings value to Context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextError {
    InitialValuesChanged,
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InitialValuesChanged => {
                f.write_str("initial simulation values cannot change after initialization")
            }
        }
    }
}

impl std::error::Error for ContextError {}
