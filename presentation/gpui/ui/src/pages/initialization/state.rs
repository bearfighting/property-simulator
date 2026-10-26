use gpui_kit::component::input::InputState;
use gpui_kit::{AppContext as _, Context, Entity, Window};
use property_simulation_lifecycle::SimulationState;
use std::rc::Rc;

use crate::{InitializationFormData, InitializationHandler};

pub(super) type StateChangedHandler = Rc<dyn Fn(SimulationState, &mut Context<InitializationPage>)>;
pub(super) type ContinueHandler = Rc<dyn Fn(&mut Context<InitializationPage>)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SubmissionState {
    Editing,
    Submitting,
    Initialized,
    Failed(String),
}

pub(crate) struct InitializationPage {
    handler: InitializationHandler,
    on_state_changed: StateChangedHandler,
    on_continue: ContinueHandler,
    pub(super) name: Entity<InputState>,
    pub(super) property_value: Entity<InputState>,
    pub(super) cash_balance: Entity<InputState>,
    pub(super) stop_after_months: Entity<InputState>,
    pub(super) stop_when_all_loans_paid_off: bool,
    pub(super) loan_enabled: bool,
    pub(super) loan_balance: Entity<InputState>,
    pub(super) annual_interest_rate: Entity<InputState>,
    pub(super) loan_term: Entity<InputState>,
    pub(super) rental_enabled: bool,
    pub(super) monthly_rent: Entity<InputState>,
    pub(super) fixed_expense_enabled: bool,
    pub(super) monthly_expenses: Entity<InputState>,
    pub(super) repairs_enabled: bool,
    pub(super) repair_maximum_amount: Entity<InputState>,
    pub(super) repair_delay_months: Entity<InputState>,
    pub(super) submission: SubmissionState,
}

impl InitializationPage {
    pub(crate) fn new(
        handler: InitializationHandler,
        on_state_changed: StateChangedHandler,
        on_continue: ContinueHandler,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
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
            on_state_changed,
            on_continue,
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
            submission: SubmissionState::Editing,
        }
    }

    pub(super) fn form_data(&self, cx: &Context<Self>) -> InitializationFormData {
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

    pub(super) const fn settings_locked(&self) -> bool {
        matches!(
            self.submission,
            SubmissionState::Submitting | SubmissionState::Initialized
        )
    }

    pub(super) fn open_simulation(&self, cx: &mut Context<Self>) {
        (self.on_continue)(cx);
    }

    pub(super) fn submit(&mut self, cx: &mut Context<Self>) {
        if self.settings_locked() {
            return;
        }

        let form = self.form_data(cx);
        let handler = self.handler.clone();
        let on_state_changed = self.on_state_changed.clone();
        self.submission = SubmissionState::Submitting;
        cx.notify();

        cx.spawn(async move |view, cx| {
            let result = handler(form).await;
            let completed_state = result.as_ref().ok().copied();
            view.update(cx, |this, cx| {
                let state = match result {
                    Ok(_) => SubmissionState::Initialized,
                    Err(error) => SubmissionState::Failed(error),
                };
                this.submission = state;
                cx.notify();
            })
            .ok();
            if let Some(state) = completed_state {
                view.update(cx, |_this, cx| (on_state_changed)(state, cx))
                    .ok();
            }
        })
        .detach();
    }
}

fn value_or_placeholder(
    input: &Entity<InputState>,
    cx: &Context<InitializationPage>,
    default: &str,
) -> String {
    let value = input.read(cx).value().to_string();
    if value.trim().is_empty() {
        default.to_owned()
    } else {
        value
    }
}
