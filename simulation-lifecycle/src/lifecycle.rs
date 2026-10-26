/// The current phase of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationState {
    /// Initial settings may be submitted.
    Initial,
    /// Monthly simulation may be performed.
    Running,
    /// Reports may be generated.
    Reporting,
}

/// A work unit associated with a lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkUnit {
    /// Set initial settings.
    InitialSettings,
    /// Simulate one month.
    MonthSimulation,
    /// Generate reports.
    Reporting,
}

impl SimulationState {
    /// Return the work unit associated with this state.
    pub const fn allowed_work(self) -> WorkUnit {
        match self {
            Self::Initial => WorkUnit::InitialSettings,
            Self::Running => WorkUnit::MonthSimulation,
            Self::Reporting => WorkUnit::Reporting,
        }
    }

    /// Return the next state in the linear lifecycle, if one exists.
    pub const fn transition(self) -> Option<Self> {
        match self {
            Self::Initial => Some(Self::Running),
            Self::Running => Some(Self::Reporting),
            Self::Reporting => None,
        }
    }
}

/// Create a lifecycle in its initial state.
pub const fn new() -> SimulationState {
    SimulationState::Initial
}
