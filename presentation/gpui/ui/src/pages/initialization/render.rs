use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::{Context, IntoElement, ParentElement as _, Render, Styled as _, Window, div, px};

use crate::{components::page_heading, pages::initialization::state::InitializationPage};

use super::sections::{render_actions, render_activities, render_general_settings};

impl Render for InitializationPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scrollbar()
            .items_center()
            .justify_start()
            .px_8()
            .py_8()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .w_full()
                    .max_w(px(1040.0))
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(page_heading(
                        "Start a simulation",
                        "Set up the opening position and the assumptions for each month.",
                        cx.theme().muted_foreground,
                    ))
                    .child(render_general_settings(self, cx))
                    .child(crate::components::section_heading(
                        "Simulation activities",
                        "Choose which activities run and configure their assumptions.",
                        cx.theme().muted_foreground,
                    ))
                    .child(render_activities(self, cx))
                    .child(render_actions(self, cx)),
            )
    }
}
