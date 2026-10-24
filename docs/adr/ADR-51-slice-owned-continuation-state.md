# ADR-51: Keep continuation state with Slice results

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [ADR-37](ADR-37-slice-journal-entry-contract.md), [ADR-48](ADR-48-monthly-snapshot-content-and-continuation.md), [Domain model](../domain-model.md)

## Context

`Snapshot` previously exposed a `LoanContinuation` field even though the loan remaining-term update was already recorded as a typed `Effect` in the Loan `SliceResult`. This made the general monthly result type own a field specific to one Slice and duplicated state already present in its result. Journal entries remain accounting facts and are not a source for state required by future calculations.

## Decision

- `Snapshot` stores each month's `SliceResult` values and does not expose Slice-specific continuation fields.
- A Slice records its typed state changes in its own result effects. For Loan, `LoanRemainingTermUpdated` records the remaining scheduled payments after an evaluated month.
- Application derives the next Slice inputs from the latest relevant Slice result effect, carrying forward the last known value when that Slice has no new effect for a month. Initial state comes from Context.

## Consequences

- Snapshot construction no longer needs previous loan continuation state or validates duplicate Loan continuation effects.
- Snapshot retains the effect as part of its `SliceResult`, preserving the state change with its source Slice and month.
- Application must implement lookup and carry-forward when its cross-month composition is designed.
