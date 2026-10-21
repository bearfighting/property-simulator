# ADR-05: Independent proposals and coordinated commit

- **Status:** Superseded by [ADR-28](ADR-28-pre-m5-crate-call-boundary-review.md)
- **Date:** 2026-10-19
- **Related:** [architecture](../architecture.md), [engineering practices](../engineering-practices.md)
- **Test vectors:** Independent proposal merge is permutation/partition invariant; successful commit coherence and proposal conflicts are tested at composition.

## Context
Multiple slices can affect shared state and accounts in the same month.

## Current status
The shared monthly coordinator described below was implemented for M3/M4 behavior and remains only as a test fixture under `crates/simulation/src/test_support/`. It is not part of the production simulation API. The current production boundary returns independently calculated Slice outputs; combining them, committing a Ledger batch, and retaining a run or history are not responsibilities implemented by the current core. ADR-28 records the replacement boundary.

## Decision
The following paragraph records the former M3/M4 coordinator behavior and its test vectors. It is not the current production module contract; see ADR-28.

Each Slice instance reads the same immutable opening context and proposes changes independently; it cannot read another instance's result. The shared coordinator accepts only combinations whose result merge is associative and commutative with an identity, rejecting incompatible shared writes before returning a result as specified in [ADR-20](ADR-20-composable-slices-and-random-sampling.md). It validates and reduces the proposals, then returns the complete successful-period result and updated Ledger without mutating caller-owned input. A fatal failure produces no successful result and stops the simulation. Core does not retain a prior boundary or promise failed-run recovery/retry or external persistence behavior (ADR-27).

## Alternatives considered
- Direct slice mutation: hides dependencies and makes independent results order-dependent.

## Consequences
Coordinator owns proposal resolution and produces the complete period result without mutating caller-owned input. Slice logic remains independently testable. Effect conflict detection uses the target resource, rather than the output key, to identify writes to shared state. The core does not retain prior month results; completed-result persistence belongs to the application layer (ADR-27).
