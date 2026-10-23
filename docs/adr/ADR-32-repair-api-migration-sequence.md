# ADR-32: Sequence repair Slice delivery and legacy API removal

- **Status:** Historical; migration complete
- **Date:** 2026-10-22
- **Scope:** Core repair Slice inputs and removal of the mixed repair API

> Historical decision record. The migration described here is complete; the mixed repair API and its compatibility path have been removed from Core.

## Context

Repair was split into a recurring `RepairOccurrence` activity and a one-time `RepairCost` activity. During the migration, the new activities had distinct configurations, samples, random-input purposes, provenance and lifecycle behavior while the monthly input/runtime still exposed the older mixed `Repair` configuration, its combined distribution and sampler.

At the time of this decision, the current workspace contained no Application crate or production callers of the old mixed repair API; remaining references were in Core and test support. The new repair inputs were implemented and `FrozenInput` no longer contains repair-only global fields.

## Decision

Deliver and validate the new repair path independently, then remove the old mixed API and its compatibility input in Core before Application integration. The new repair configurations and execution do not require global repair-only values.

The Core cleanup was completed. Tests used the new repair inputs, retaining accounting, provenance and lifecycle assertions while deleting tests that only covered obsolete mixed behavior. This records the migration sequence and does not set current module boundaries.

## Alternatives

- Delete every old repair type in the initial Slice implementation: rejected at that time because it combined new Slice behavior with a broad test migration. The new Slice behavior is now separately validated, so the cleanup can proceed as a distinct Core task.
- Keep the old mixed behavior as a permanent parallel model: rejected because it would preserve two competing repair contracts and keep global repair settings in monthly inputs.

## Consequences

- The new Slice runtime can be completed and tested without waiting for SQLite or Application implementation.
- The temporary compatibility path was removed with the Core cleanup; there is no supported legacy entry point.
- The mixed repair API migration is complete. The former Slice runtime refactor plan has been removed; repair semantics that remain in scope are recorded in the domain model.

## Verification examples

- A month containing only rental or the new repair activities is constructible and calculable from the current Slice inputs.
- `cargo test -p property-simulation-models` and `cargo test -p property-simulation-simulation` pass after the Core cleanup.
