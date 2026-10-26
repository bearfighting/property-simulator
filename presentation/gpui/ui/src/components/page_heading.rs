use gpui_kit::{IntoElement, ParentElement as _, Styled as _, div};

pub(crate) fn page_heading(
    title: &'static str,
    description: &'static str,
    muted_foreground: gpui_kit::Hsla,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_3xl().child(title))
        .child(
            div()
                .text_base()
                .text_color(muted_foreground)
                .child(description),
        )
}
