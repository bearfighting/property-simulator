use gpui_kit::component::{
    ActiveTheme as _, Disableable as _,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{Context, IntoElement, ParentElement as _, Styled as _, div};

use crate::pages::initialization::state::{InitializationPage, SubmissionState};

pub(crate) fn render(
    page: &InitializationPage,
    cx: &Context<InitializationPage>,
) -> impl IntoElement {
    let (status, is_error, button_label) = match &page.submission {
        SubmissionState::Editing => (None, false, "Create simulation"),
        SubmissionState::Submitting => (Some("Creating simulation…"), false, "Creating…"),
        SubmissionState::Initialized => (
            Some("Simulation initialized. Month 0 is ready."),
            false,
            "Simulation created",
        ),
        SubmissionState::Failed(error) => (Some(error.as_str()), true, "Create simulation"),
    };

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .children(status.map(|status| {
            let color = if is_error {
                cx.theme().danger_foreground
            } else {
                cx.theme().muted_foreground
            };
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .text_color(color)
                .child(status.to_owned())
        }))
        .child(
            Button::new("create-simulation")
                .primary()
                .label(button_label)
                .disabled(matches!(
                    page.submission,
                    SubmissionState::Submitting | SubmissionState::Initialized
                ))
                .on_click(cx.listener(|this, _, window, cx| this.confirm_submit(window, cx))),
        )
}
