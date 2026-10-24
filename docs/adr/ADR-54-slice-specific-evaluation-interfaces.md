# ADR-54: Slice-specific evaluation interfaces

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md), [ADR-46](ADR-46-explicit-slice-construction-and-application-composition.md), [ADR-34](ADR-34-simulation-random-root-lifecycle.md)

## Context

Deterministic Slices do not need random input, while RepairOccurrence and RepairCost consume an explicit random source. A single `SliceInstance::evaluate` signature either forced deterministic Slices to accept an irrelevant optional dependency or made random input mandatory for every Slice. Storing the source on Slice instances would couple their state and lifetime to the caller's random stream.

## Decision

- Define two narrow evaluation capabilities: `DeterministicSlice::calculate` for calculations without randomness, and `StochasticSlice::simulate` for behavior that consumes an explicit random source.
- Deterministic Slices implement only `DeterministicSlice`; stochastic Slices implement only `StochasticSlice` and take `&mut dyn RandomInput` explicitly.
- All Slices return the shared `SliceResult` contract, but return their own concrete error types.
- Application constructs the concrete Slice types it selected and invokes the matching trait method. Core does not add a dispatcher or a universal Slice trait to coordinate them.
- Random streams remain owned and allocated by the caller; Slice instances never store them.

## Alternatives considered

- Pass `Option<&mut dyn RandomInput>` to every Slice: rejected because deterministic Slices still expose an irrelevant argument and stochastic Slices can fail at runtime due to a missing source.
- Store the random source on each Slice: rejected because it couples the Slice instance to a borrowed or owned mutable stream and its lifetime.

## Consequences

- Deterministic calculations and stochastic simulations have explicit, separate trait bounds and cannot accidentally omit or consume randomness.
- Application handles concrete Slice error types or maps them at its own use-case boundary.
- Each Slice exposes its concrete error type; a new Slice does not need a variant in a shared evaluation error enum.
