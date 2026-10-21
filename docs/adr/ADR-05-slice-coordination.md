# ADR-05: Independent proposals and coordinated commit

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [architecture](../architecture.md), [engineering practices](../engineering-practices.md)
- **Test vectors:** Independent proposal merge is permutation/partition invariant; successful commit coherence and proposal conflicts are tested at composition.

## Context
Multiple slices can affect shared state and accounts in the same month.

## Decision
Each Slice instance reads the same immutable opening context and proposes changes independently; it cannot read another instance's result. The shared coordinator accepts only combinations whose result merge is associative and commutative with an identity, rejecting incompatible shared writes before commit as specified in [ADR-20](ADR-20-composable-slices-and-random-sampling.md). It validates and reduces the proposals, then returns the complete successful-period result and updated Ledger without mutating caller-owned state. Run control records that result and its snapshot as one successful in-memory period boundary. A failed period produces no successful result and leaves caller-visible in-memory state at the last successful boundary; run control marks a fatal run failed and does not allow it to continue in place. Recovery/retry of that failed run and consistency of external persistence after a fatal failure are not promised.

## Alternatives considered
- Direct slice mutation: hides dependencies and makes independent results order-dependent.

## Consequences
Coordinator owns proposal resolution and produces the complete period result without mutating caller-owned state; run control owns adding successful results and snapshots to the in-memory history. Slice logic remains independently testable. Effect conflict detection uses the target resource, rather than the output key, to identify writes to shared state. In-memory failure atomicity preserves the last successful boundary; it does not imply recovery of a failed run or guarantees for external persistence.
