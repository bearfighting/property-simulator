use gpui_kit::{Hsla, IntoElement, ParentElement as _, Styled as _, div, px};

use crate::{
    CashFlowViewCategory, ReportCashFlowAccountView, ReportCashFlowCategoryView, ReportPeriodView,
    components::PageTheme,
};

pub(super) fn period_performance(period: &ReportPeriodView, theme: PageTheme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_lg().child("Period performance"))
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!("Month {}–{}", period.start_month, period.end_month)),
                ),
        )
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_4()
                .child(amount_panel(
                    "Income",
                    &period.income,
                    "Total income",
                    &period.total_income,
                    theme,
                ))
                .child(amount_panel(
                    "Expenses",
                    &period.expenses,
                    "Total expenses",
                    &period.total_expenses,
                    theme,
                )),
        )
        .child(metric_card("Net income", &period.net_income, theme))
}

pub(super) fn cumulative_cash_flow(
    period: &ReportPeriodView,
    theme: PageTheme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(div().text_lg().child("Cumulative cash flow"))
        .child(
            div()
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .bg(theme.panel)
                .p_5()
                .flex()
                .flex_col()
                .gap_3()
                .child(cash_flow_header(theme.muted_foreground))
                .children(period.cash_flow_categories.iter().map(|category| {
                    cash_flow_category_row(
                        category,
                        &period.cash_flow_accounts,
                        theme,
                    )
                }))
                .child(div().h(px(1.0)).w_full().bg(theme.border))
                .child(cash_flow_totals(period, theme.muted_foreground)),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child("Cash flow follows cash movements. Loan principal repayments are financing outflows, not expenses."),
        )
}

fn cash_flow_header(muted: Hsla) -> impl IntoElement {
    div()
        .grid()
        .grid_cols(3)
        .child(
            div()
                .text_sm()
                .text_color(muted)
                .child("Category / account"),
        )
        .child(
            div()
                .text_sm()
                .text_color(muted)
                .text_right()
                .child("Cash inflows"),
        )
        .child(
            div()
                .text_sm()
                .text_color(muted)
                .text_right()
                .child("Cash outflows"),
        )
}

fn cash_flow_category_row(
    category: &ReportCashFlowCategoryView,
    accounts: &[ReportCashFlowAccountView],
    theme: PageTheme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .grid()
                .grid_cols(3)
                .child(div().text_sm().child(category_label(category.category)))
                .child(div().text_sm().text_right().child(category.inflows.clone()))
                .child(
                    div()
                        .text_sm()
                        .text_right()
                        .child(category.outflows.clone()),
                ),
        )
        .children(
            accounts
                .iter()
                .filter(|account| account.category == category.category)
                .map(|account| {
                    div()
                        .grid()
                        .grid_cols(3)
                        .border_t_1()
                        .border_color(theme.border)
                        .pt_2()
                        .child(
                            div()
                                .pl_4()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(account.account.clone()),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .text_right()
                                .child(account.inflows.clone()),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .text_right()
                                .child(account.outflows.clone()),
                        )
                }),
        )
}

fn cash_flow_totals(period: &ReportPeriodView, muted: Hsla) -> impl IntoElement {
    div()
        .grid()
        .grid_cols(3)
        .child(div().text_sm().child("Total cash movement"))
        .child(
            div()
                .text_sm()
                .text_right()
                .child(period.cash_inflows.clone()),
        )
        .child(
            div()
                .text_sm()
                .text_color(muted)
                .text_right()
                .child(period.cash_outflows.clone()),
        )
}

fn amount_panel(
    title: &'static str,
    rows: &[crate::ReportAmountView],
    total_label: &'static str,
    total: &str,
    theme: PageTheme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .p_5()
        .child(div().text_base().child(title))
        .children(
            rows.iter()
                .map(|row| amount_row(&row.label, &row.amount, theme.muted_foreground)),
        )
        .child(div().h(px(1.0)).w_full().bg(theme.border))
        .child(amount_row(total_label, total, theme.muted_foreground))
}

fn amount_row(label: &str, amount: &str, muted: Hsla) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(div().text_sm().text_color(muted).child(label.to_owned()))
        .child(div().text_sm().child(amount.to_owned()))
}

fn metric_card(label: &'static str, amount: &str, theme: PageTheme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .p_4()
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(div().text_lg().child(amount.to_owned()))
}

const fn category_label(category: CashFlowViewCategory) -> &'static str {
    match category {
        CashFlowViewCategory::Operating => "Operating",
        CashFlowViewCategory::Investing => "Investing",
        CashFlowViewCategory::Financing => "Financing",
    }
}
