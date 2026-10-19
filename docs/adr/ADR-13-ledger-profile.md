# ADR-13: Household property ledger profile

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-07](ADR-07-monthly-cashflow.md)
- **Test vectors:** [L-01–L-07](../test-vectors.md#ledger-vectors)

## Context
The initial accounting model supports a household/personal owner of property, not a business accounting or tax system.

## Decision
Use double-entry accounting and the account types assets, liabilities, equity, income, and expenses. The focused chart covers in-scope cash, property reference value, mortgage and other included liabilities, opening equity, rental income, and property operating expenses (interest, tax, insurance, maintenance/repairs, utilities). Debit increases assets/expenses; credit increases liabilities/equity/income. Contra asset and contra expense accounts reverse those normal balance directions. Opening assets and liabilities are supplied facts; opening equity is the balancing net worth, not a simulated-period posting. If the user supplies net worth, it must equal opening assets less opening liabilities. Store opening balances as opening facts, not generated pre-start or period transactions. Modeled period accrual and cash settlement occur in the same monthly close.

## Alternatives considered
- Corporate chart, depreciation, payroll, or tax accounting: outside personal-property scope.
- Plugging opening differences to miscellaneous income: misstates period performance.

## Consequences
Balances and statements derive from opening balances and posted entries. Principal repayment reduces cash and liability, not expense; unpaid accrued interest is recognized as interest expense with the unpaid portion credited to the mortgage liability. Future accounting profiles require an explicit versioned decision.
