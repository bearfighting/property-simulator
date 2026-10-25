//! GPUI presentation for simulation initialization.
//!
//! This crate depends only on the UI toolkit. Its public submission contract
//! contains plain form values and a displayable result, not Application or
//! persistence types.

use std::{future::Future, pin::Pin, rc::Rc};

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _,
    button::{Button, ButtonVariants as _},
    form::{Field, Form},
    input::{Input, InputState},
    scroll::ScrollableElement as _,
    switch::Switch,
};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowBounds, WindowOptions, application, assets, div, init, open_window, px, size,
};

/// User-entered values for the initial simulation screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializationFormData {
    pub name: String,
    pub property_value: String,
    pub cash_balance: String,
    pub stop_after_months: String,
    pub stop_when_all_loans_paid_off: bool,
    pub loan_enabled: bool,
    pub loan_balance: String,
    pub annual_interest_rate_percent: String,
    pub loan_term_months: String,
    pub rental_enabled: bool,
    pub monthly_rent: String,
    pub fixed_expense_enabled: bool,
    pub monthly_expenses: String,
    pub repairs_enabled: bool,
    pub repair_maximum_amount: String,
    pub repair_delay_months: String,
}

/// Result returned by the host after it coordinates initialization.
pub type SubmissionResult = Result<(), String>;

/// Async host callback. The future is local to GPUI's foreground executor.
pub type InitializationHandler =
    Rc<dyn Fn(InitializationFormData) -> Pin<Box<dyn Future<Output = SubmissionResult> + 'static>>>;

/// Open the simulation initialization window with a host-provided submit handler.
pub fn run_initialization_ui(handler: InitializationHandler) {
    application().with_assets(assets::Assets).run(move |cx| {
        init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1120.0), px(860.0)), cx)),
            window_min_size: Some(size(px(820.0), px(700.0))),
            ..WindowOptions::default()
        };
        open_window(options, cx, {
            let handler = handler.clone();
            move |window, cx| {
                cx.new(|window_cx| InitializationView::new(handler, window, window_cx))
            }
        })
        .expect("failed to open the simulation initialization window");
    });
}

struct InitializationView {
    handler: InitializationHandler,
    name: Entity<InputState>,
    property_value: Entity<InputState>,
    cash_balance: Entity<InputState>,
    stop_after_months: Entity<InputState>,
    stop_when_all_loans_paid_off: bool,
    loan_enabled: bool,
    loan_balance: Entity<InputState>,
    annual_interest_rate: Entity<InputState>,
    loan_term: Entity<InputState>,
    rental_enabled: bool,
    monthly_rent: Entity<InputState>,
    fixed_expense_enabled: bool,
    monthly_expenses: Entity<InputState>,
    repairs_enabled: bool,
    repair_maximum_amount: Entity<InputState>,
    repair_delay_months: Entity<InputState>,
    status: Option<String>,
    status_is_error: bool,
    submitting: bool,
    initialized: bool,
}

impl InitializationView {
    fn new(handler: InitializationHandler, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("My rental property"));
        let property_value = cx.new(|cx| InputState::new(window, cx).placeholder("1000000"));
        let cash_balance = cx.new(|cx| InputState::new(window, cx).placeholder("0"));
        let stop_after_months = cx.new(|cx| InputState::new(window, cx).placeholder("No limit"));
        let loan_balance = cx.new(|cx| InputState::new(window, cx).placeholder("700000"));
        let annual_interest_rate = cx.new(|cx| InputState::new(window, cx).placeholder("5"));
        let loan_term = cx.new(|cx| InputState::new(window, cx).placeholder("240"));
        let monthly_rent = cx.new(|cx| InputState::new(window, cx).placeholder("2500"));
        let monthly_expenses = cx.new(|cx| InputState::new(window, cx).placeholder("500"));
        let repair_maximum_amount = cx.new(|cx| InputState::new(window, cx).placeholder("5000"));
        let repair_delay_months = cx.new(|cx| InputState::new(window, cx).placeholder("1"));

        Self {
            handler,
            name,
            property_value,
            cash_balance,
            stop_after_months,
            stop_when_all_loans_paid_off: true,
            loan_enabled: true,
            loan_balance,
            annual_interest_rate,
            loan_term,
            rental_enabled: true,
            monthly_rent,
            fixed_expense_enabled: true,
            monthly_expenses,
            repairs_enabled: false,
            repair_maximum_amount,
            repair_delay_months,
            status: None,
            status_is_error: false,
            submitting: false,
            initialized: false,
        }
    }

    fn form_data(&self, cx: &Context<Self>) -> InitializationFormData {
        InitializationFormData {
            name: value_or_placeholder(&self.name, cx, "My rental property"),
            property_value: value_or_placeholder(&self.property_value, cx, "1000000"),
            cash_balance: value_or_placeholder(&self.cash_balance, cx, "0"),
            stop_after_months: self.stop_after_months.read(cx).value().to_string(),
            stop_when_all_loans_paid_off: self.stop_when_all_loans_paid_off,
            loan_enabled: self.loan_enabled,
            loan_balance: value_or_placeholder(&self.loan_balance, cx, "700000"),
            annual_interest_rate_percent: value_or_placeholder(&self.annual_interest_rate, cx, "5"),
            loan_term_months: value_or_placeholder(&self.loan_term, cx, "240"),
            rental_enabled: self.rental_enabled,
            monthly_rent: value_or_placeholder(&self.monthly_rent, cx, "2500"),
            fixed_expense_enabled: self.fixed_expense_enabled,
            monthly_expenses: value_or_placeholder(&self.monthly_expenses, cx, "500"),
            repairs_enabled: self.repairs_enabled,
            repair_maximum_amount: value_or_placeholder(&self.repair_maximum_amount, cx, "5000"),
            repair_delay_months: value_or_placeholder(&self.repair_delay_months, cx, "1"),
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.submitting {
            return;
        }

        let form = self.form_data(cx);
        let handler = self.handler.clone();
        self.submitting = true;
        self.status = Some("Creating simulation…".to_owned());
        self.status_is_error = false;
        cx.notify();

        cx.spawn(async move |view, cx| {
            let result = handler(form).await;
            view.update(cx, |this, cx| {
                this.submitting = false;
                let (message, is_error) = match result {
                    Ok(()) => {
                        this.initialized = true;
                        (
                            "Simulation initialized. Month 0 is ready.".to_owned(),
                            false,
                        )
                    }
                    Err(error) => (error, true),
                };
                this.status = Some(message);
                this.status_is_error = is_error;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Render for InitializationView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let button_label = if self.submitting {
            "Creating…"
        } else if self.initialized {
            "Simulation created"
        } else {
            "Create simulation"
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .px_8()
                    .py_4()
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().popover)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_6()
                            .child(div().text_lg().child("Property Simulation"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Simulation setup"),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(if self.initialized { "Month 0 ready" } else { "New simulation" }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_grow_1()
                    .overflow_y_scrollbar()
                    .justify_center()
                    .px_8()
                    .py_8()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(1040.0))
                            .flex()
                            .flex_col()
                            .gap_6()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(div().text_3xl().child("Start a simulation"))
                                    .child(
                                        div()
                                            .text_base()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Set up the opening position and the assumptions for each month."),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_col()
                                    .gap_6()
                                    .child(
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
                                                cx,
                                            ))
                                            .child(
                                                Form::new()
                                                    .child(Field::new().label("Simulation name").child(Input::new(&self.name)))
                                                    .child(Field::new().label("Property value · CAD").child(Input::new(&self.property_value)))
                                                    .child(Field::new().label("Cash at start · CAD").child(Input::new(&self.cash_balance)))
                                                    .child(Field::new().label("Stop after months · optional").child(Input::new(&self.stop_after_months)))
                                                    .child(Field::new().label_indent(false).child(
                                                        Switch::new("stop-when-loans-paid-off")
                                                            .label("Stop when all loans are paid off")
                                                            .checked(self.stop_when_all_loans_paid_off)
                                                            .on_change(cx.listener(|this, next, _, cx| {
                                                                this.stop_when_all_loans_paid_off = *next;
                                                                cx.notify();
                                                            })),
                                                    )),
                                            ),
                                    )
                            .child(section_heading(
                                "Simulation activities",
                                "Choose which activities run and configure their assumptions.",
                                cx,
                            ))
                            .child(
                                div()
                                    .grid()
                                    .grid_cols(2)
                                    .gap_6()
                                    .child(activity_panel(
                                        "Loan",
                                        "Opening mortgage balance and repayment terms.",
                                        Switch::new("loan-enabled")
                                            .label("Enabled")
                                            .checked(self.loan_enabled)
                                            .on_change(cx.listener(|this, next, _, cx| {
                                                this.loan_enabled = *next;
                                                cx.notify();
                                            })),
                                        Form::new()
                                            .child(Field::new().label("Mortgage balance · CAD").child(Input::new(&self.loan_balance).disabled(!self.loan_enabled)))
                                            .child(Field::new().label("Annual interest rate · %").child(Input::new(&self.annual_interest_rate).disabled(!self.loan_enabled)))
                                            .child(Field::new().label("Mortgage term · months").child(Input::new(&self.loan_term).disabled(!self.loan_enabled))),
                                        cx,
                                    ))
                                    .child(activity_panel(
                                        "Rental income",
                                        "Recurring rent collected each month.",
                                        Switch::new("rental-enabled")
                                            .label("Enabled")
                                            .checked(self.rental_enabled)
                                            .on_change(cx.listener(|this, next, _, cx| {
                                                this.rental_enabled = *next;
                                                cx.notify();
                                            })),
                                        Form::new().child(
                                            Field::new()
                                                .label("Monthly rent · CAD")
                                                .child(Input::new(&self.monthly_rent).disabled(!self.rental_enabled)),
                                        ),
                                        cx,
                                    ))
                                    .child(activity_panel(
                                        "Operating expenses",
                                        "Recurring property expenses each month.",
                                        Switch::new("fixed-expense-enabled")
                                            .label("Enabled")
                                            .checked(self.fixed_expense_enabled)
                                            .on_change(cx.listener(|this, next, _, cx| {
                                                this.fixed_expense_enabled = *next;
                                                cx.notify();
                                            })),
                                        Form::new().child(
                                            Field::new()
                                                .label("Monthly expenses · CAD")
                                                .child(Input::new(&self.monthly_expenses).disabled(!self.fixed_expense_enabled)),
                                        ),
                                        cx,
                                    ))
                                    .child(activity_panel(
                                        "Repairs",
                                        "Random repair events and their settlement timing.",
                                        Switch::new("repairs-enabled")
                                            .label("Enabled")
                                            .checked(self.repairs_enabled)
                                            .on_change(cx.listener(|this, next, _, cx| {
                                                this.repairs_enabled = *next;
                                                cx.notify();
                                            })),
                                        Form::new()
                                            .child(Field::new().label("Maximum repair cost · CAD").child(Input::new(&self.repair_maximum_amount).disabled(!self.repairs_enabled)))
                                            .child(Field::new().label("Settlement delay · months").child(Input::new(&self.repair_delay_months).disabled(!self.repairs_enabled))),
                                        cx,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_4()
                                    .children(self.status.as_ref().map(|status| {
                                        let color = if self.status_is_error {
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
                                            .disabled(self.submitting || self.initialized)
                                            .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
                                    ),
                            ),
                    )
                ),
            )
    }
}

fn section_heading(
    title: &'static str,
    description: &'static str,
    cx: &Context<InitializationView>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_lg().child(title))
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(description),
        )
}

fn activity_panel(
    title: &'static str,
    description: &'static str,
    toggle: impl IntoElement,
    fields: impl IntoElement,
    cx: &Context<InitializationView>,
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
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_3()
                .child(section_heading(title, description, cx))
                .child(toggle),
        )
        .child(fields)
}

fn value_or_placeholder(
    input: &Entity<InputState>,
    cx: &Context<InitializationView>,
    default: &str,
) -> String {
    let value = input.read(cx).value().to_string();
    if value.trim().is_empty() {
        default.to_owned()
    } else {
        value
    }
}
