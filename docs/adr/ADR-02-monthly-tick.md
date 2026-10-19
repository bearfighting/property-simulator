# ADR-02: One monthly transactional tick

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [architecture](../architecture.md), [domain model](../domain-model.md)
- **Test vectors:** N/A; month-boundary behavior is covered by M3 lifecycle tests.

## Context
Slices need a stable shared period for proposals, postings, and snapshots.

## Decision
One calendar month is one inclusive-period tick and one transactional close. A 20-year example is 240 ticks, not a duration limit.

## Alternatives considered
- Daily or variable tick sizes: unnecessary for the first monthly model.

## Consequences
Sub-month accrual is out of scope; each month closes before the next opens.
