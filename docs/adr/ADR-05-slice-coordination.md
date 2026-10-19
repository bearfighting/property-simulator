# ADR-05: Independent proposals and coordinated commit

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [architecture](../architecture.md), [engineering practices](../engineering-practices.md)
- **Test vectors:** Independent proposal merge is permutation/partition invariant; successful commit coherence and proposal conflicts are tested at composition.

## Context
Multiple slices can affect shared state and accounts in the same month.

## Decision
Slices read context and propose changes. A shared coordinator validates conflicts and commits world effects, ledger transactions, events, and snapshots as one successful period result. Unexpected runtime failures are fatal; the system does not promise rollback, recovery, or persistent-state consistency after such a failure. Independent, conflict-free proposals follow the associative and commutative merge contract in [ADR-20](ADR-20-composable-slices-and-random-sampling.md).

## Alternatives considered
- Direct slice mutation: hides dependencies and makes independent results order-dependent.

## Consequences
Coordinator owns resolution and the successful-period commit boundary; slice logic remains independently testable. Effect conflict detection uses the target resource, rather than the output key, to identify writes to shared state. Commit coherence on successful results does not imply recovery guarantees after fatal runtime failure.
