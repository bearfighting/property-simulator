use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{Context, IntoElement, ParentElement as _, Styled as _, div};

use crate::app::shell::AppShell;

pub(crate) fn placeholder_page(
    title: &'static str,
    description: &'static str,
    cx: &Context<AppShell>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_grow_1()
        .items_center()
        .justify_center()
        .p_8()
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .child(div().text_2xl().child(title))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(description),
                ),
        )
}
