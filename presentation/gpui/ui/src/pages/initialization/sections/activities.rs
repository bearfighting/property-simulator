use gpui_kit::component::{
    ActiveTheme as _,
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
                    .on_change(cx.listener(|this, next, _, cx| {
                        this.loan_enabled = *next;
                        cx.notify();
                    })),
                Form::new()
                    .child(
                        Field::new()
                            .label("Mortgage balance · CAD")
                            .child(Input::new(&page.loan_balance).disabled(!page.loan_enabled)),
                    )
                    .child(
                        Field::new().label("Annual interest rate · %").child(
                            Input::new(&page.annual_interest_rate).disabled(!page.loan_enabled),
                        ),
                    )
                    .child(
                        Field::new()
                            .label("Mortgage term · months")
                            .child(Input::new(&page.loan_term).disabled(!page.loan_enabled)),
                    ),
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
                    .on_change(cx.listener(|this, next, _, cx| {
                        this.rental_enabled = *next;
                        cx.notify();
                    })),
                Form::new().child(
                    Field::new()
                        .label("Monthly rent · CAD")
                        .child(Input::new(&page.monthly_rent).disabled(!page.rental_enabled)),
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
                    .on_change(cx.listener(|this, next, _, cx| {
                        this.fixed_expense_enabled = *next;
                        cx.notify();
                    })),
                Form::new().child(Field::new().label("Monthly expenses · CAD").child(
                    Input::new(&page.monthly_expenses).disabled(!page.fixed_expense_enabled),
                )),
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
                    .on_change(cx.listener(|this, next, _, cx| {
                        this.repairs_enabled = *next;
                        cx.notify();
                    })),
                Form::new()
                    .child(Field::new().label("Maximum repair cost · CAD").child(
                        Input::new(&page.repair_maximum_amount).disabled(!page.repairs_enabled),
                    ))
                    .child(Field::new().label("Settlement delay · months").child(
                        Input::new(&page.repair_delay_months).disabled(!page.repairs_enabled),
                    )),
                cx.theme().popover,
                cx.theme().border,
                cx.theme().muted_foreground,
            )),
    )
}
