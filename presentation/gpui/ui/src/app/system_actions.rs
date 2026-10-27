use gpui_kit::component::{
    Disableable as _,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{
    App, ClickEvent, Context, Entity, IntoElement, ParentElement as _, Styled as _, Window, div,
};
use property_simulation_lifecycle::SimulationState;

use super::{shell::AppShell, status_bar};

/// Renders application-level actions that are independent from the selected page.
pub(super) fn render(
    state: SimulationState,
    is_simulating_month: bool,
    app: Entity<AppShell>,
    cx: &Context<AppShell>,
) -> impl IntoElement {
    let end_app = app.clone();
    let reset_app = app;

    div()
        .flex()
        .items_center()
        .gap_2()
        .child(status_bar::render(state, cx))
        .child(
            Button::new("end-simulation")
                .danger()
                .label("End simulation")
                .disabled(state != SimulationState::Running || is_simulating_month)
                .on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                    end_app.update(cx, |shell, cx| shell.confirm_stop(window, cx));
                }),
        )
        .child(
            Button::new("reset-simulation")
                .secondary()
                .label("Reset simulation")
                .disabled(state == SimulationState::Initial || is_simulating_month)
                .on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                    reset_app.update(cx, |shell, cx| shell.confirm_reset(window, cx));
                }),
        )
}
