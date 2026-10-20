# ADR-14: Mortgage interest and payment convention

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related:** [domain model](../domain-model.md), [roadmap](../implementation-roadmap.md), [ADR-22](ADR-22-m3-run-inputs-snapshot-and-cashflow-contract.md)
- **Test vectors:** [M-01–M-11](../test-vectors.md#mortgage-vectors)

## Context
The mortgage milestone needs an unambiguous monthly convention and invalid-input boundary. The same convention applies to other fixed-rate, fully amortizing installment loans included in M3; it does not define revolving or non-amortizing debt behavior.

## Decision
The required loan calculation inputs are the opening principal balance, a positive integer remaining amortization term in months, and nominal annual interest rate. M3 supports fixed-rate, fully amortizing loans; the rate is fixed for the applicable schedule. The activity schedule determines the effective month. The model derives a level monthly payment; the user does not supply a competing payment amount. Use monthly rate `i = r / 12` and payment `P = B × i / (1 - (1 + i)^(-n))`, where `B` is the balance, `r` is the nominal annual rate, and `n` is the remaining number of monthly payments. For zero rate, use `P = B / n`. Calculate with decimal fixed-point precision and round the scheduled payment to the currency minor unit using positive half-up rounding. The derived payment remains level through the schedule; normal monthly principal reduction does not recalculate it. An explicit balance, rate, or remaining-term update effective in a later month recalculates the payment from that month's opening balance and remaining term. A saved month-end snapshot retains the derived payment and remaining term needed to continue the schedule.

The effective month is also the first interest and payment month: a loan or term change effective on the first day of a month accrues interest for that full month and makes its first payment at that month's close. This applies to a loan active in the opening state at the simulation start month as well as a loan introduced during the run. For each active month, calculate interest on opening principal for the full month using `i`; round posted interest to the currency minor unit half-up. Apply the scheduled payment interest-first, then principal. The final payment is adjusted to exactly settle the remaining principal and that month's posted interest, preventing a residual balance or overpayment due to rounding. For valid positive balance, non-negative rate, and positive finite term, the derived payment covers interest; unpaid-interest capitalization and negative amortization are not part of this convention. Zero rate and no-loan state are valid. Negative annual rates, non-positive opening principal for an active loan, and non-positive remaining term are structural input errors. New loans and term changes have an effective `YearMonth` supplied by the activity schedule and take effect on its first day; day-level effective dates are unsupported.

For M3 Ledger integration, an opening loan balance is an opening liability fact, not a fabricated historical disbursement. A loan originated during the simulated period posts its actual proceeds as a financing cash inflow against the liability. For each monthly payment, debit interest expense by posted interest `I_m` and debit loan liability by principal paid `Q_m`; credit cash by interest paid `I_m` and principal paid `Q_m` as separately classified postings. The postings balance, and the loan liability changes by `-Q_m`, matching the closing balance. Interest is operating cash flow; principal repayment is financing cash flow.

## Alternatives considered
- Recalculate the scheduled payment from each month's new balance and remaining term: rejected because it changes the installment each month instead of preserving a level-payment schedule.
- Require the user to supply a separate monthly payment: rejected because it can conflict with the supplied remaining term and interest rate.
- Supporting negative rates or day-level partial accrual/effective dates: deferred pending an explicit versioned convention; negative-rate inputs are rejected under this convention.

## Consequences
Use exact decimal arithmetic, not binary floating point. Tests must assert the derived payment, month-by-month interest and principal split, final-payment adjustment, closing balance, balanced Ledger postings, and reconciliation between loan continuation balances and Ledger liabilities. The payment formula and rounding convention are part of the model version. See vectors for zero rate, ordinary amortization, final payment, and rounding boundaries.
