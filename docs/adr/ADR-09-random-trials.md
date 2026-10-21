# ADR-09: Explicit sampling inputs and diagnostics

- **Status:** Partially superseded by [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md) and [ADR-29](ADR-29-layered-calculation-and-application-boundaries.md)
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-15](ADR-15-random-streams-and-branches.md)
- **Test vectors:** M3 fixed-seed diagnostic fixtures; exact product trajectory is not invariant.

## Context
This ADR originally combined random-sampling traceability with trial, snapshot, and branch behavior. The latter runtime concepts are outside the current design; sampling and replay expectations remain relevant.

## Decision
Pass required random inputs through explicit caller-provided sampling interfaces, keeping probability distributions separate from sampling and stream allocation as specified in [ADR-21](ADR-21-m3-run-randomness-and-persistence.md). Exact repeated output is not a product invariant. Fixed seeds may be used for tests and diagnostics. Do not require a persisted trial identity or a new-trial workflow for this calculation boundary. The application version is the only software version identifier.

## Alternatives considered
- Requiring identical output across independent calls: would turn test diagnostics into a product replay guarantee.

## Consequences
Trial recording, snapshot continuation, branching, and re-simulation behavior in the original decision are historical and are not current requirements. The M3 random-source and semantic substream details documented by [ADR-21](ADR-21-m3-run-randomness-and-persistence.md) describe the existing sampling adapter and its tests; they do not establish a run-management or replay workflow.
