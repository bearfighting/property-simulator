# ADR-05: Independent proposals and coordinated commit

- **Status:** Superseded by [ADR-28](ADR-28-pre-m5-crate-call-boundary-review.md)
- **Date:** 2026-10-19
- **Related:** [architecture](../architecture.md), [engineering practices](../engineering-practices.md)
- **Test vectors:** Independent proposal merge is permutation/partition invariant; former successful-commit composition and proposal-conflict coverage remains in historical test support.

## Context
Multiple slices can affect shared state and accounts in the same month.

## Historical implementation status
At the time this ADR was superseded, the shared monthly coordinator described below had been moved to `crates/simulation/src/test_support/`; the production API exposed independently calculated Slice outputs and did not combine them or commit a Ledger batch. That was the pre-M6 boundary, not the current production contract. ADR-29 later assigned stateless single-month integration to M6, and ADR-30 defines its input, settlement, and result-audit contract. The old M3/M4 coordinator remains test-only.

## Decision
The following paragraph records the former M3/M4 coordinator behavior and its test vectors. It is not the current production module contract; the current M6 integration boundary is defined by ADR-29 and ADR-30.

Each Slice instance reads the same immutable opening context and proposes changes independently; it cannot read another instance's result. The shared coordinator accepts only combinations whose result merge is associative and commutative with an identity, rejecting incompatible shared writes before returning a result as specified in [ADR-20](ADR-20-composable-slices-and-random-sampling.md). It validates and reduces the proposals, then returns the complete successful-period result and updated Ledger without mutating caller-owned input. A fatal failure produces no successful result and stops the simulation. Core does not retain a prior boundary or promise failed-run recovery/retry or external persistence behavior (ADR-27).

## Alternatives considered
- Direct slice mutation: hides dependencies and makes independent results order-dependent.

## Historical consequences
In the former M3/M4 implementation, the coordinator owned proposal resolution and produced the complete period result without mutating caller-owned input. Slice logic remained independently testable. Effect conflict detection used the target resource, rather than the output key, to identify writes to shared state. These responsibilities are divided between independent Core functions and the future M6 monthly integration boundary; completed-state persistence belongs to the later Application/Data Access flow (ADR-27/29).
