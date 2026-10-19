# Ledger and mortgage test vectors

These hand-calculated vectors are the shared initial verification baseline for Ledger and mortgage calculations. Amounts are CAD unless stated; all values are exact dollars. Posted values use cents and positive half-up rounding. `Dr` and `Cr` mean debit and credit. A transaction balances when debits equal credits. ADRs reference IDs here to avoid duplicated arithmetic.

## Ledger vectors

The minimal accounts used below are Cash (asset), Property (asset), Mortgage payable (liability), Opening equity (equity), Rental income (income), Interest expense, Repairs expense, and Property tax expense (expenses). Asset/expense normal balances are debit; liability/equity/income normal balances are credit.

| ID | Transaction | Debit | Credit | Check |
|---|---|---|---|---|
| L-01 | Opening balances: property $300,000, cash $20,000, mortgage $200,000; opening net worth $120,000 | Property 300,000; Cash 20,000 | Mortgage 200,000; Opening equity 120,000 | 320,000 = 320,000; opening-balance record, not a simulated-period transaction or cash flow |
| L-02 | Monthly payment $1,500: principal $1,000, interest $500 | Mortgage 1,000; Interest expense 500 | Cash 1,500 | 1,500 = 1,500; loan balance falls by 1,000 |
| L-03 | Repair paid $800 | Repairs expense 800 | Cash 800 | 800 = 800 |
| L-04 | Rent received $2,000 | Cash 2,000 | Rental income 2,000 | 2,000 = 2,000 |
| L-05 | Property tax paid $300 | Property tax expense 300 | Cash 300 | 300 = 300 |
| L-06 | Reconciliation after L-02 to L-05 | Opening balances plus listed debits/credits | — | Cash = 20,000 - 1,500 - 800 + 2,000 - 300 = 19,400; operating cash flow = -500 - 800 + 2,000 - 300 = 400; financing cash flow = -1,000; net cash change = -600; mortgage = 199,000; net income = 400; assets 319,400 = liabilities 199,000 + opening equity 120,000 + net income 400 |
| L-07 | Interest due $100, payment $50; $50 unpaid interest capitalizes (M-03) | Interest expense 100 | Cash 50; Mortgage payable 50 | 100 = 100; expense is fully recognized, $50 operating cash outflow is recorded, liability rises by $50 without a cash flow |

M1.3 opening-balance checks: L-01 is represented by four opening facts (property debit $300,000; cash debit $20,000; mortgage credit $200,000; equity credit $120,000), with derived totals of assets $320,000, liabilities $200,000, equity/net worth $120,000. The opening facts remain outside period transactions and cash flow. A negative-net-worth case uses assets debit $80, liabilities credit $100, and equity debit $20; the debit equity balance is valid and the opening set still balances. A normal asset credit balance is rejected; a contra-asset credit balance reduces net assets. Reject an unbalanced set, negative amount, duplicate or unknown account, income/expense account, mixed currency, or declared net worth that differs from assets less liabilities.

### L-08 — Negative cash-position continuity

| Period activity | Debit | Credit | Check |
|---|---|---|---|
| Month 1: repairs $15 | Repairs expense 15 | Cash position 15 | Balanced |
| Month 2: rent received $3 | Cash position 3 | Rental income 3 | Balanced |

Opening cash position is $10 Dr. Month 1 closes at cash position $5 Cr (signed −$5), not a generated overdraft/loan; month 2 closes at $2 Cr (signed −$2). Cumulative property operating cash flow is −$12; cumulative net income is −$12; modeled assets are signed cash −$2; liabilities $0; equity −$2. Both month-end states reconcile and simulation continues.

L-08 is a scoped property-ledger result. The negative cash position does not assert that a real bank paid the shortfall, that the household borrowed, or that any off-ledger household funding occurred.

### Money range vectors

Money uses a signed `i128` coefficient and scale up to 28. Parsing a 38-digit integer and adding it to another amount must remain exact when the result fits; adding one to `i128::MAX` must return an explicit arithmetic overflow. A value with scale 29 is rejected. No accepted or calculated amount may wrap or be clamped.

## Mortgage vectors

For every active loan vector, rate is nominal annual rate divided by 12. Compute with decimal precision, post interest rounded to cents half-up, then apply actual payment interest-first and principal-second; actual payment cannot exceed principal plus posted interest.

| ID | Opening principal | Annual rate | Planned payment | Interest | Actual payment | Principal paid | Interest capitalized | Closing principal |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| M-01 Zero rate | 1,000.00 | 0% | 100.00 | 0.00 | 100.00 | 100.00 | 0.00 | 900.00 |
| M-02 Normal month | 10,000.00 | 12% | 200.00 | 100.00 | 200.00 | 100.00 | 0.00 | 9,900.00 |
| M-03 Payment below interest | 10,000.00 | 12% | 50.00 | 100.00 | 50.00 | 0.00 | 50.00 | 10,050.00 |
| M-04 Payment equals interest | 10,000.00 | 12% | 100.00 | 100.00 | 100.00 | 0.00 | 0.00 | 10,000.00 |
| M-05 Payment above interest | 10,000.00 | 12% | 200.00 | 100.00 | 200.00 | 100.00 | 0.00 | 9,900.00 |
| M-06 Capitalization carried forward | 10,000.00 | 12% | 50.00 (month 1); 100.50 (month 2) | 100.00; 100.50 | 50.00; 100.50 | 0.00; 0.00 | 50.00; 0.00 | 10,050.00; 10,050.00 |
| M-07 Final payment cap | 100.00 | 12% | 1,000.00 | 1.00 | 101.00 | 100.00 | 0.00 | 0.00 |
| M-08 Half-cent rounding | 0.50 | 12% | 1.00 | 0.01 | 0.51 | 0.50 | 0.00 | 0.00 |
| M-09 No loan | Absent / paid off | — | — | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| M-10 Low balance below half cent interest | 0.01 | 6% | 0.01 | 0.00 | 0.01 | 0.01 | 0.00 | 0.00 |
| M-11 Zero payment | 10,000.00 | 12% | 0.00 | 100.00 | 0.00 | 0.00 | 100.00 | 10,100.00 |

M-03/04/05 use the same principal/rate to distinguish payment allocation boundaries. M-06 verifies a shortfall is capitalized and the next month accrues on the increased opening principal. M-08 is the positive half-up midpoint: $0.50 × 1% = $0.005 → $0.01. M-10 is $0.01 × 0.5% = $0.00005 → $0.00. M-11 confirms zero payment is allowed and all posted interest capitalizes; this is not delinquency/default.

### Structural input errors

Reject negative annual interest rate, non-positive principal on an active loan, negative planned payment, invalid currency precision, and terms effective other than at a month start. Zero annual rate and an absent/paid-off loan are valid. These vectors define calculation expectations; Ledger vectors are implemented with M1, and mortgage vectors with the mortgage milestone, not as placeholder tests in M0.
