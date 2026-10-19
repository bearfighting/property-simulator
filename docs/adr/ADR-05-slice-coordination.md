# ADR-05: Independent proposals and coordinated commit

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [architecture](../architecture.md), [engineering practices](../engineering-practices.md)
- **Test vectors:** N/A; proposal conflicts and atomicity are tested at M4 composition.

## Context
Multiple slices can affect shared state and accounts in the same month.

## Decision
Slices read context and propose changes. A shared coordinator validates conflicts and atomically commits world effects, ledger transactions, events, and snapshots.

## Alternatives considered
- Direct slice mutation: permits partial commits and order-dependent hidden effects.

## Consequences
Coordinator owns resolution and the unit-of-work boundary; slice logic remains independently testable.
