use gpui_kit::{IntoElement, ParentElement as _, Styled as _, div};

pub(crate) fn activity_panel(
    title: &'static str,
    description: &'static str,
    toggle: impl IntoElement,
    fields: impl IntoElement,
    popover: gpui_kit::Hsla,
    border: gpui_kit::Hsla,
    muted_foreground: gpui_kit::Hsla,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .rounded_lg()
        .bg(popover)
        .border_1()
        .border_color(border)
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_3()
                .child(crate::components::section_heading(
                    title,
                    description,
                    muted_foreground,
                ))
                .child(toggle),
        )
        .child(fields)
}
