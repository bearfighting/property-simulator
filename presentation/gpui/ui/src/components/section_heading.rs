use gpui_kit::{IntoElement, ParentElement as _, Styled as _, div};

pub(crate) fn section_heading(
    title: &'static str,
    description: &'static str,
    muted_foreground: gpui_kit::Hsla,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_lg().child(title))
        .child(
            div()
                .text_sm()
                .text_color(muted_foreground)
                .child(description),
        )
}
