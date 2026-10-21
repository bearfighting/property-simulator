# Ledger and mortgage vectors

These values specify domain behavior and remain usable by future tests. They are behavioral examples, not implementation status reports.

## Ledger

Normal balances: assets/expenses debit; liabilities/equity/income credit. Amounts below are CAD.

| ID | Input/postings | Required result |
|---|---|---|
| L-01 Opening facts | Property 300,000 Dr; cash 20,000 Dr; mortgage 200,000 Cr; opening equity 120,000 Cr | Opening balances balance at 320,000; no period transaction or cash flow is created. |
| L-02 Mortgage payment | Mortgage 1,000 Dr; interest expense 500 Dr; cash 1,500 Cr | Balanced; principal reduces liability by 1,000. |
| L-03 Repair | Repairs expense 800 Dr; cash 800 Cr | Balanced; operating cash flow −800. |
| L-04 Rent | Cash 2,000 Dr; rental income 2,000 Cr | Balanced; operating cash flow +2,000. |
| L-05 Property tax | Property-tax expense 300 Dr; cash 300 Cr | Balanced; operating cash flow −300. |
| L-06 Projection | Apply L-02 through L-05 to L-01 | Cash 19,400; operating flow +400; financing flow −1,000; mortgage 199,000; net income 400; assets 319,400 = liabilities 199,000 + equity 120,000 + income 400. |
| L-07 Accrual capability | Interest expense 100 Dr; cash 50 Cr; interest payable 50 Cr | Balanced; operating cash flow −50. This is a generic Ledger capability, not the mortgage convention. |
| L-08 Negative cash position | Opening cash 10; month 1 repair 15; month 2 rent 3 | Signed closing cash −5 then −2; no implicit borrowing; cumulative operating flow/net income −12. |
| L-09 Detail projection | Rent +100 operating, repair −30 operating, +5/−5 investing in one transaction, financing +10 | Return transaction/category signed amounts with transaction IDs and optional source; include both range endpoints, exclude opening cash, and reconcile to category totals. |

An unbalanced opening set or transaction, duplicate/unknown account, mixed currency, invalid account role, negative posting amount or checked arithmetic overflow is rejected. Negative net worth is valid when opening facts balance.

## Fixed-rate amortizing mortgage

For opening principal `B`, nominal annual rate `r`, monthly rate `j = r / 12`, and remaining term `n`, scheduled payment is `P = B × j / (1 - (1 + j)^(-n))`; at zero rate, `P = B / n`. Round scheduled payment and monthly interest to cents half-up. Interest accrues on opening monthly principal. Payment occurs at month end, pays interest before principal; the final payment is adjusted to clear the balance.

| ID | Opening principal | Annual rate | Term | Payment | Interest | Principal | Closing balance |
|---|---:|---:|---:|---:|---:|---:|---:|
| M-01 | 1,000.00 | 0% | 10 | 100.00 | 0.00 | 100.00 | 900.00 |
| M-02 | 10,000.00 | 12% | 12 | 888.49 | 100.00 | 788.49 | 9,211.51 |
| M-03 | 100.00 | 12% | 1 | 101.00 | 1.00 | 100.00 | 0.00 |
| M-04 | 0.50 | 12% | 1 | 0.51 | 0.01 | 0.50 | 0.00 |
| M-05 | 0.01 | 6% | 1 | 0.01 | 0.00 | 0.01 | 0.00 |
| M-06 | 9,211.51 | 12% | 11 remaining | 888.49 | 92.12 | 796.37 | 8,415.14 |
| M-07 | 879.67 | 12% | 1 remaining, continuing M-02 schedule | 888.49 scheduled; 888.47 actual | 8.80 | 879.67 | 0.00 |

Reject an active loan with non-positive principal, non-positive remaining term, negative rate, or a derived scheduled payment that rounds to zero. A zero-rate loan and an absent or paid-off loan are valid. Each loan's closing balance must reconcile to its own liability account; principal is financing cash flow and paid interest is operating cash flow.
