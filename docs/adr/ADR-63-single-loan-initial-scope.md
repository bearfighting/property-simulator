# ADR-63: Support one loan per simulation in the initial scope

- **Status:** Accepted
- **Date:** 2026-10-24
- **Related:** [Domain model](../domain-model.md), [ADR-51](ADR-51-slice-owned-continuation-state.md), [SQLite schema](../../data-access/sqlite/schema.sql)
- **Verification:** Initial persistence allows one effective Loan configuration per simulation/month; Loan Effect has no per-loan identifier; the loan liability account is the Ledger source for that loan's balance.

## Context

The current settings schema has one Loan configuration per simulation and
effective month. The current application design does not define stable loan
identities or per-loan configuration collections. Adding an identifier only to
the Loan Effect would leave the settings, account mapping, and stop-condition
model inconsistent. Property refinancing or cash-out remortgaging would also
introduce broader household financing activity beyond the current property
simulation scope.

## Decision

- The initial simulation scope supports one loan per simulation.
- Do not model refinancing, cash-out remortgaging, or new loans secured against
  the property in the initial scope.
- `LoanRemainingTermUpdated` applies to that simulation's sole Loan and does
  not carry a separate loan ID or account code.
- The loan-specific liability account and its Ledger entries provide the
  balance facts; the Effect carries only the remaining scheduled term.
- Household-level financing is outside this model. If it ever becomes a product
  goal, it requires its own domain design for financing activity and household
  assets and liabilities.

## Alternatives considered

- Add a loan ID to `LoanRemainingTermUpdated` now: rejected because no matching
  loan identity exists in settings or Application state.
- Treat multiple Loan Slice instances as supported without explicit identity:
  rejected because their settings and continuation effects could not be
  associated reliably across months.

## Consequences

- The current one-row-per-simulation/month Loan settings key matches the
  supported scope.
- Household financing is not modeled as an additional Loan Slice case. It
  would change the product's accounting scope and its domain contracts.
