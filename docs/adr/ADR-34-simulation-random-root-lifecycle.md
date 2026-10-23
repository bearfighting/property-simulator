# ADR-34: Simulation random root lifecycle

- **Status:** Accepted
- **Date:** 2026-10-22
- **Related:** [Domain model](../domain-model.md), [Architecture](../architecture.md), [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md)
- **Verification:** Confirm Core calculation interfaces consume explicit random input and do not access system entropy; confirm Context initialization stores the generated root.

## Context

Random input must be available to stochastic Slice calculations without making domain rules responsible for system entropy, stream allocation, or hidden random state. A simulation may continue across multiple months, so the random root needs a clear lifetime and persistence location.

## Decision

- Application obtains one random root from the system when it creates the initial Context and persists that root with the initial user settings.
- The root remains available in Context for the lifetime of the simulation.
- Random engine derives per-Slice random input from the root and explicit semantic identity. Domain calculations consume that input and do not access system entropy or allocate cross-month streams.
- Snapshot does not duplicate random stream state when streams are derived from the Context root and semantic identity.

## Alternatives considered

- Generate a new root for each month: not selected because it gives the simulation no single random root across its lifetime and complicates consistent stream derivation.
- Persist evolving random state in each Snapshot: not selected for stateless derivation from a persistent root and semantic identity.
- Let Core obtain system entropy: not selected because it would introduce system access into domain calculation.

## Consequences

- Context must include a persistable representation of the random root before Application initialization is implemented.
- Random engine interfaces must accept the root and semantic identity explicitly; Slice evaluation receives only its needed random input.
- Whether a completed simulation retains or deletes its root is deferred to the post-completion retention policy, which is outside current scope.
