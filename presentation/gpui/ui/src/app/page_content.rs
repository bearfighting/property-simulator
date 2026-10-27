use property_simulation_lifecycle::SimulationState;

use super::page::AppPage;

/// Textual guidance for a route, derived from the current lifecycle state.
/// Keeping this mapping out of the shell leaves navigation and layout
/// independent from lifecycle copy as new work units are added.
pub(super) struct PageContent {
    pub(super) title: &'static str,
    pub(super) description: &'static str,
}

pub(super) const fn for_route(page: AppPage, state: SimulationState) -> PageContent {
    let (title, description) = match (page, state) {
        (AppPage::MonthlySimulation, SimulationState::Initial) => (
            "Monthly simulation",
            "Complete initial settings before starting monthly simulation.",
        ),
        (AppPage::MonthlySimulation, SimulationState::Running) => (
            "Monthly simulation",
            "Monthly results and controls will appear here.",
        ),
        (AppPage::MonthlySimulation, SimulationState::Stopped) => (
            "Monthly simulation",
            "This run has ended. Open Reporting to view its results.",
        ),
        (AppPage::Summary, SimulationState::Initial) => (
            "Reporting",
            "Complete initial settings and end the simulation before viewing its report.",
        ),
        (AppPage::Summary, SimulationState::Running) => (
            "Reporting",
            "End the run to view its report. If no month was completed, it will show the Month 0 opening position.",
        ),
        (AppPage::Summary, SimulationState::Stopped) => (
            "Reporting",
            "Simulation-wide results and reports will appear here.",
        ),
        (AppPage::Initialization, _) => ("Start a simulation", ""),
    };

    PageContent { title, description }
}
