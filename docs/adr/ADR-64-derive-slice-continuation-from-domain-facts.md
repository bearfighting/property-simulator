# ADR-64: Derive Slice continuation from domain facts

- **Status:** Accepted
- **Date:** 2026-10-24
- **Supersedes:** Effect-related continuation portions of [ADR-51](ADR-51-slice-owned-continuation-state.md) and [ADR-60](ADR-60-core-simulation-contracts-without-aggregates.md)
- **Related:** [ADR-62](ADR-62-month-scoped-event-facts.md), [ADR-63](ADR-63-single-loan-initial-scope.md), [Domain model](../domain-model.md)

## Context

The Core `Effect` values currently represented the Loan's remaining scheduled
term and the month of a settled repair cost. In the current model, neither is
independent state: Loan remaining term follows the fixed schedule and current
simulation Month, while the repair baseline is the Month of the latest
`RepairCostSettled` Event. Loan payments continue while the Ledger has a
positive loan balance; delinquency, deferred payments, and default are outside
scope.

Persisting both values would duplicate facts already represented by the
simulation timeline, Events, configuration, and Ledger.

## Decision

- Remove the generic Core `Effect` type and the `effects` collection from
  `SliceResult`.
- Do not define Effect DTOs or persist monthly Effects. Remove the draft
  SQLite `month_effects` table.
- Application derives the current repair interval input from the latest
  persisted `RepairCostSettled` Event month (including zero-cost settlements).
- Application derives the scheduled remaining loan term from the original
  term and current Month. It invokes Loan only while the Ledger loan balance
  is positive; a paid-off loan produces no later payment.
- If a future domain behavior introduces continuation state that cannot be
  derived from Month, Events, configuration, or Ledger, model that state for
  that behavior explicitly before deciding whether it needs persistence.

## Alternatives considered

- Keep Effects as generic typed state-change records: rejected because current
  variants duplicate derivable facts and add a persistence concept without a
  current use case.
- Preserve only one of the current Effect variants: rejected because both
  values already have authoritative domain facts from which they can be
  derived.

## Consequences

- `SliceResult` contains Month, Slice kind, Events, and JournalEntries only.
- Monthly storage does not need an Effect table, DTO, mapping, or serialization
  format.
- Application remains responsible for assembling explicit inputs from the
  simulation timeline, persisted event history, and Ledger projections.
- A genuinely new multi-month behavior, such as a repair activity with
  progress beyond its occurrence and settlement facts, may require its own
  explicit persisted state; this decision does not preclude that design.

## Verification examples

- A zero-amount repair settlement still emits `RepairCostSettled`; its Month
  resets the next occurrence interval without a separate state-change record.
- A Loan payment reduces the Ledger liability. Subsequent Months continue
  payments while the liability is positive; after it reaches zero, no Loan
  result is produced.
