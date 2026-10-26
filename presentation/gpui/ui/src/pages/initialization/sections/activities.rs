use gpui_kit::component::{
    ActiveTheme as _, Disableable as _,
    form::{Field, Form},
    input::Input,
    switch::Switch,
};
use gpui_kit::{Context, IntoElement, ParentElement as _, Styled as _, div};

use crate::{components::activity_panel, pages::initialization::state::InitializationPage};

pub(crate) fn render(
    page: &InitializationPage,
    cx: &Context<InitializationPage>,
) -> impl IntoElement {
    let settings_locked = page.settings_locked();
    div().flex().flex_col().gap_4().child(
        div()
            .grid()
            .grid_cols(2)
            .gap_6()
            .child(activity_panel(
                "Loan",
                "Opening mortgage balance and repayment terms.",
                Switch::new("loan-enabled")
                    .label("Enabled")
                    .checked(page.loan_enabled)
                    .disabled(settings_locked)
                    .on_change(cx.listener(|this, next, _, cx| {
                        if this.settings_locked() {
                            return;
                        }
                        this.loan_enabled = *next;
                        cx.notify();
                    })),
                Form::new()
                    .child(
                        Field::new().label("Mortgage balance · CAD").child(
                            Input::new(&page.loan_balance)
                                .disabled(settings_locked || !page.loan_enabled),
                        ),
                    )
                    .child(
                        Field::new().label("Annual interest rate · %").child(
                            Input::new(&page.annual_interest_rate)
                                .disabled(settings_locked || !page.loan_enabled),
                        ),
                    )
                    .child(Field::new().label("Mortgage term · months").child(
                        Input::new(&page.loan_term).disabled(settings_locked || !page.loan_enabled),
                    )),
                cx.theme().popover,
                cx.theme().border,
                cx.theme().muted_foreground,
            ))
            .child(activity_panel(
                "Rental income",
                "Recurring rent collected each month.",
                Switch::new("rental-enabled")
                    .label("Enabled")
                    .checked(page.rental_enabled)
                    .disabled(settings_locked)
                    .on_change(cx.listener(|this, next, _, cx| {
                        if this.settings_locked() {
                            return;
                        }
                        this.rental_enabled = *next;
                        cx.notify();
                    })),
                Form::new().child(
                    Field::new().label("Monthly rent · CAD").child(
                        Input::new(&page.monthly_rent)
                            .disabled(settings_locked || !page.rental_enabled),
                    ),
                ),
                cx.theme().popover,
                cx.theme().border,
                cx.theme().muted_foreground,
            ))
            .child(activity_panel(
                "Operating expenses",
                "Recurring property expenses each month.",
                Switch::new("fixed-expense-enabled")
                    .label("Enabled")
                    .checked(page.fixed_expense_enabled)
                    .disabled(settings_locked)
                    .on_change(cx.listener(|this, next, _, cx| {
                        if this.settings_locked() {
                            return;
                        }
                        this.fixed_expense_enabled = *next;
                        cx.notify();
                    })),
                Form::new().child(
                    Field::new().label("Monthly expenses · CAD").child(
                        Input::new(&page.monthly_expenses)
                            .disabled(settings_locked || !page.fixed_expense_enabled),
                    ),
                ),
                cx.theme().popover,
                cx.theme().border,
                cx.theme().muted_foreground,
            ))
            .child(activity_panel(
                "Repairs",
                "Random repair events and their settlement timing.",
                Switch::new("repairs-enabled")
                    .label("Enabled")
                    .checked(page.repairs_enabled)
                    .disabled(settings_locked)
                    .on_change(cx.listener(|this, next, _, cx| {
                        if this.settings_locked() {
                            return;
                        }
                        this.repairs_enabled = *next;
                        cx.notify();
                    })),
                Form::new()
                    .child(
                        Field::new().label("Maximum repair cost · CAD").child(
                            Input::new(&page.repair_maximum_amount)
                                .disabled(settings_locked || !page.repairs_enabled),
                        ),
                    )
                    .child(
                        Field::new().label("Settlement delay · months").child(
                            Input::new(&page.repair_delay_months)
                                .disabled(settings_locked || !page.repairs_enabled),
                        ),
                    ),
                cx.theme().popover,
                cx.theme().border,
                cx.theme().muted_foreground,
            )),
    )
}
