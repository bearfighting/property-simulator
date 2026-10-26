use gpui_kit::component::{
    ActiveTheme as _, Disableable as _,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{Context, IntoElement, ParentElement as _, Styled as _, div};

use crate::pages::initialization::state::InitializationPage;

pub(crate) fn render(
    page: &InitializationPage,
    cx: &Context<InitializationPage>,
) -> impl IntoElement {
    let button_label = if page.submitting {
        "Creating…"
    } else if page.initialized {
        "Simulation created"
    } else {
        "Create simulation"
    };

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .children(page.status.as_ref().map(|status| {
            let color = if page.status_is_error {
                cx.theme().danger_foreground
            } else {
                cx.theme().muted_foreground
            };
            div().text_sm().text_color(color).child(status.clone())
        }))
        .child(
            Button::new("create-simulation")
                .primary()
                .label(button_label)
                .disabled(page.submitting || page.initialized)
                .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
        )
}
