use gpui_kit::component::{
    ActiveTheme as _,
    form::{Field, Form},
    input::Input,
    switch::Switch,
};
use gpui_kit::{Context, IntoElement, ParentElement as _, Styled as _, div};

use crate::{components::section_heading, pages::initialization::state::InitializationPage};

pub(crate) fn render(
    page: &InitializationPage,
    cx: &Context<InitializationPage>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .rounded_lg()
        .bg(cx.theme().popover)
        .border_1()
        .border_color(cx.theme().border)
        .child(section_heading(
            "General settings",
            "Name and opening balances for Month 0. Currency: CAD.",
            cx.theme().muted_foreground,
        ))
        .child(
            Form::new()
                .child(
                    Field::new()
                        .label("Simulation name")
                        .child(Input::new(&page.name)),
                )
                .child(
                    Field::new()
                        .label("Property value · CAD")
                        .child(Input::new(&page.property_value)),
                )
                .child(
                    Field::new()
                        .label("Cash at start · CAD")
                        .child(Input::new(&page.cash_balance)),
                )
                .child(
                    Field::new()
                        .label("Stop after months · optional")
                        .child(Input::new(&page.stop_after_months)),
                )
                .child(
                    Field::new().label_indent(false).child(
                        Switch::new("stop-when-loans-paid-off")
                            .label("Stop when all loans are paid off")
                            .checked(page.stop_when_all_loans_paid_off)
                            .on_change(cx.listener(|this, next, _, cx| {
                                this.stop_when_all_loans_paid_off = *next;
                                cx.notify();
                            })),
                    ),
                ),
        )
}
