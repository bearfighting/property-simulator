use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{Context, IntoElement, ParentElement as _, Styled as _, div};
use property_simulation_lifecycle::SimulationState;

use super::shell::AppShell;

/// Displays the lifecycle state without taking ownership of its transitions.
pub(super) fn render(state: SimulationState, cx: &Context<AppShell>) -> impl IntoElement {
    let label = match state {
        SimulationState::Initial => "Initial",
        SimulationState::Running => "Running",
        SimulationState::Stopped => "Stopped",
    };

    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().popover)
        .child(
            div()
                .size_2()
                .rounded_full()
                .bg(cx.theme().muted_foreground),
        )
        .child(div().text_sm().child(label))
}
