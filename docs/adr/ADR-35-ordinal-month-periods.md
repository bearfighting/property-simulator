# ADR-35: Ordinal month periods

- **Status:** Accepted; simulation start semantics refined by [ADR-39](ADR-39-context-settings-and-month-zero-start.md)
- **Date:** 2026-10-23
- **Related:** [Domain model](../domain-model.md), [Architecture](../architecture.md), [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md)
- **Verification examples:** Check that Month(0) is valid and first, checked arithmetic rejects overflow and movement before month zero, and each completed Snapshot carries its Month key.

## Context

The simulation calculates sequential monthly periods and does not model calendar dates or timestamps. A calendar `YearMonth` adds year and calendar-month semantics that are not required by the current calculation contract. A month value is also useful as a key for the unique completed result of that period within one simulation.

## Decision

- Represent a simulation period as a zero-based ordinal `Month`; the first calculated period is Month 0. Preserve this numbering in stored and displayed domain values without an implicit offset.
- Support ordering, hashing, checked increment/offset operations, and elapsed-month calculation. Do not impose a product-level maximum duration.
- Use Month as the period key for a monthly Snapshot. It is unique only within one simulation and is not a globally unique ID.
- Keep serialization formats outside the domain type and map Month to a DTO in Data Access.
- Do not add a generic ID crate now. Add a business identity only when a concrete reference requirement establishes its meaning.
- If a future behavior needs calendar-season alignment, model that calendar input explicitly rather than inferring it from Month.

## Alternatives

- Keep `YearMonth`: not selected because the current simulation period has no calendar year or date semantics.
- Store only a month-of-year value from 1 to 12: rejected because it cannot order multiple years of simulation or identify a monthly result.
- Use an untyped integer throughout Core: rejected because it mixes period arithmetic with arbitrary numbers and carries no month-specific operations.

## Consequences

- Core period consumers use `Month`, including Slice inputs, Ledger transaction periods, and Snapshot keys.
- Each simulation starts at Month 0; opening Ledger facts precede it, and continuation increments from the latest Snapshot with checked arithmetic.
- Calendar timestamps, if needed for technical audit metadata, remain outside this domain value.
