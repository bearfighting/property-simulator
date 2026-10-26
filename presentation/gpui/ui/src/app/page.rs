#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AppPage {
    Initialization,
    MonthlySimulation,
    Summary,
}

impl AppPage {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Initialization => "Setup",
            Self::MonthlySimulation => "Monthly simulation",
            Self::Summary => "Summary",
        }
    }
}
