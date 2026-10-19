# ADR-13: Household property ledger profile

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-07](ADR-07-monthly-cashflow.md)
- **Test vectors:** [L-01–L-07](../test-vectors.md#ledger-vectors)

## Context
The initial accounting model supports a household/personal owner of property, not a business accounting or tax system.

## Decision
Use double-entry accounting and the account types assets, liabilities, equity, income, and expenses. The focused chart covers in-scope cash, property reference value, mortgage and other included liabilities, opening equity, rental income, and property operating expenses (interest, tax, insurance, maintenance/repairs, utilities). Debit increases assets/expenses; credit increases liabilities/equity/income. Contra asset and contra expense accounts reverse those normal balance directions. Account type determines normal balance and validation; `AccountCode` is a separate readable code for grouping/order and never determines posting behavior. Use broad code ranges by statement class: 1000–1999 assets, 2000–2999 liabilities, 3000–3999 equity, 4000–4999 income, and 5000–5999 expenses. Within assets, order accounts by liquidity; within liabilities, order by expected settlement timing where applicable. Leave gaps for additions and do not renumber existing codes when adding a leaf account. This is a project convention, not a GAAP-mandated numbering scheme.

In the implementation, `AccountCategory` represents the five classes and `BalanceSide` represents debit or credit. `AccountCategory::normal_balance(contra)` derives debit for asset/expense and credit for liability/equity/income; a contra flag reverses only asset and expense. `AccountCode` is an opaque numeric value accepting only 1000–5999, displays as four digits, sorts numerically, and must fall within the category's range. An `Account` has a caller-supplied opaque `AccountId`, code, category, contra flag, and non-empty name. A `ChartOfAccounts` rejects duplicate IDs and duplicate codes. These checks are local to one chart; they do not define posting or opening-balance rules. `CashFlowCategory` is a separate purpose classification (`Operating`, `Investing`, `Financing`) to be supplied for cash-flow components/postings, not stored on an account. Thus a cash account can participate in all three classifications depending on the transaction component.

Unit tests alongside the accounting modules cover these representation rules; the shared L-01–L-07 vectors continue to cover posting and projection behavior as those milestones are implemented.

Opening assets and liabilities are supplied facts; opening equity is the balancing net worth, not a simulated-period posting. If the user supplies net worth, it must equal opening assets less opening liabilities. Negative net worth is valid when balances reconcile; it does not stop a run. Store opening balances as opening facts, not generated pre-start or period transactions. Modeled period accrual and cash settlement occur in the same monthly close.

Opening-balance records contain an account ID, debit/credit side, and non-negative amount. Only asset, liability, and equity accounts may carry an opening balance; omitted accounts are zero. A normal asset cannot carry an abnormal credit balance; contra assets may carry their normal credit balance and reduce the asset total. A validated opening set uses one currency, has unique known account IDs, and balances total debits to total credits. Debit equity balances represent negative equity/net worth. Asset, liability, equity, and net-worth totals are derived from account categories and sides. Optional user-declared net worth is checked against derived assets less liabilities; the equity balance remains part of the balancing opening facts. These records create no period transaction or cash-flow component.

The initial cash-flow projection uses the direct method: derive actual cash receipts/payments from cash-account postings, explicitly classifying cash-flow components as operating, investing, or financing. Rent received and property operating expenses (including interest paid) are operating; property purchases/sales are investing; loan principal and owner contributions are financing. A combined cash posting may be split across classifications when its components have different purposes. Non-cash interest capitalization is not a cash flow. This is a simulation presentation convention, not statutory reporting advice.

## Alternatives considered
- Corporate chart, depreciation, payroll, or tax accounting: outside personal-property scope.
- Plugging opening differences to miscellaneous income: misstates period performance.
- Indirect cash flow as the initial projection: requires reconciliation adjustments for non-cash and timing differences that are not part of the initial model.

## Consequences
Balances and statements derive from opening balances and posted entries. Principal repayment reduces cash and liability, not expense; unpaid accrued interest is recognized as interest expense with the unpaid portion credited to the mortgage liability. Negative equity is represented by a debit balance in equity, without allowing an otherwise invalid negative cash/property asset balance. Future accounting profiles require an explicit versioned decision.

## References
- The U.S. Department of Education's [Federal Student Aid Handbook, Accounting Systems](https://fsapartners.ed.gov/knowledge-center/fsa-handbook/2025-2026/vol4/appx-accounting-systems) describes its account numbers as arbitrary and says those numbers are not required for institutions' own ledgers; the ranges above are likewise this project's convention.
- [IAS 1](https://www.ifrs.org/issued-standards/list-of-standards/ias-1-presentation-of-financial-statements.html) is an example of a reporting standard that addresses statement presentation/classification. This project does not claim IFRS or GAAP compliance.
