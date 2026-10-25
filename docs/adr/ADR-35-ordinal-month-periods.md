# ADR-35: Ordinal month periods

- **Status:** Accepted; Month 0/Month 1 semantics refined by [ADR-55](ADR-55-ledger-journal-as-source-of-balances.md)
- **Date:** 2026-10-23
- **Related:** [Domain model](../domain-model.md), [Architecture](../architecture.md), [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md)
- **Verification examples:** Check that Month(0) is the initialization boundary, Month(1) is the first calculated month, checked arithmetic rejects overflow and movement before month zero, and completed Snapshots carry Month keys starting at 1.

## Context

The simulation calculates sequential monthly periods and does not model calendar dates or timestamps. A calendar `YearMonth` adds year and calendar-month semantics that are not required by the current calculation contract. A month value is also useful as a key for the unique completed result of that period within one simulation.

## Decision

- Represent time within a simulation as a zero-based ordinal `Month`. Month 0 is the initialization boundary and Month 1 is the first calculated period. Preserve this numbering in stored and displayed domain values without an implicit offset.
- Store the ordinal as `u16`. This gives a technical ceiling of 65,535 months (over 5,400 years), far beyond the intended simulation horizon, while allowing compact DTOs. This is a representation bound, not a configured simulation duration.
- Support ordering, hashing, checked increment/offset operations, and elapsed-month calculation. Do not impose a product-level maximum duration.
- Use Month as the period key for a monthly Snapshot. Month 0 cannot have a completed-month Snapshot; it is unique only within one simulation and is not a globally unique ID.
- Keep serialization formats outside the domain type and map Month to a DTO in Data Access.
- Do not add a generic ID crate now. Add a business identity only when a concrete reference requirement establishes its meaning.
- If a future behavior needs calendar-season alignment, model that calendar input explicitly rather than inferring it from Month.

## Alternatives

- Keep `YearMonth`: not selected because the current simulation period has no calendar year or date semantics.
- Store only a month-of-year value from 1 to 12: rejected because it cannot order multiple years of simulation or identify a monthly result.
- Use an untyped integer throughout Core: rejected because it mixes period arithmetic with arbitrary numbers and carries no month-specific operations.

## Consequences

- Core period consumers use `Month`, including Slice inputs, Ledger transaction periods, and Snapshot keys.
- Each simulation initializes at Month 0. Opening facts are journaled at Month 0; the first calculation is Month 1. Continuation increments from the latest completed month with checked arithmetic.
- Calendar timestamps, if needed for technical audit metadata, remain outside this domain value.
