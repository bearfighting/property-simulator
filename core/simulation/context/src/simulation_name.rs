/// Required display name for one simulation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationName(String);

impl SimulationName {
    /// Create a name, trimming surrounding whitespace and rejecting an empty value.
    pub fn new(value: impl Into<String>) -> Result<Self, SimulationNameError> {
        let value = value.into();
        let value = value.trim();
        if value.is_empty() {
            return Err(SimulationNameError::Empty);
        }

        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Failure to create a non-empty simulation name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationNameError {
    Empty,
}

impl std::fmt::Display for SimulationNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("simulation name cannot be empty"),
        }
    }
}

impl std::error::Error for SimulationNameError {}
