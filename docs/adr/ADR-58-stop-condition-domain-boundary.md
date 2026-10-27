# ADR-58: Stop condition domain boundary

- **Status:** Accepted
- **Date:** 2026-10-24
- **Related:** [ADR-56](ADR-56-stop-conditions-and-month-boundaries.md), [ADR-70](ADR-70-immutable-simulation-configuration.md), [ADR-71](ADR-71-in-memory-simulation-lifecycle-coordinator.md), [ADR-72](ADR-72-month-bounded-duration-settings.md), [Domain model](../domain-model.md)
- **Verification examples:** `AfterMonths(1)` allows Month 1 and stops before Month 2; an empty list does not stop.

## Context

Stop conditions were defined inside the Core `context` crate alongside user
settings, simulation metadata, and random-root state. Their values and boundary
calculation are independently useful and should not depend on that aggregate.
Determining whether every loan is paid off requires current simulation facts,
but stop-condition evaluation need not inspect a Ledger or a Loan Slice.

## Decision

- Define `StopCondition` and `should_stop` in a dedicated `stop-condition`
  crate. The crate depends only on `Month`.
- Evaluate conditions using the latest completed Month and an explicit
  `all_loans_paid_off` fact supplied by the caller. Stop when any condition is
  satisfied; an empty condition list does not stop.
- `AfterMonths(n)` is satisfied when the completed Month is at least `n`.
- Manual stopping is not a stop condition. The composition layer changes the
  lifecycle state in response to a user command; only configured automatic
  conditions are evaluated here.
- Keep lifecycle coordination outside the crate. Application decides when to
  check conditions and whether to start the next month; the crate does not own
  settings, Context, Ledger, Slice, or persistence.

## Alternatives considered

- Keep stop conditions inside `Context`: rejected because condition evaluation
  is independent of Context and the aggregate is being reconsidered.
- Make the crate inspect Ledger or loan state directly: rejected because this
  would couple a small policy calculation to accounting and Slice details.

## Consequences

- Application derives the loan payoff fact and passes it with the completed
  Month when checking the configured conditions.
- The condition value is immutable after simulation initialization; manual
  stop remains outside this crate and the persistence model.
