# ADR-16: Configuration and run controls

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [roadmap](../implementation-roadmap.md), [domain model](../domain-model.md), [ADR-01](ADR-01-monthly-run-control.md), [ADR-04](ADR-04-scenario-and-overrides.md), [ADR-15](ADR-15-random-streams-and-branches.md)
- **Test vectors:** M3 lifecycle/configuration tests cover these semantics.

## Context
Initial facts, run bounds, and changing monthly assumptions have different lifetimes and provenance.

## Decision
Scenario contains opening facts, future assumptions, enabled slices, and explicit versions. Run controls contain an inclusive start month and at least one ending mechanism: an optional inclusive end month, configured month-end termination conditions, or external per-month continue/stop control. Evaluate termination conditions after the month's close and snapshot; any one satisfied condition ends the run (OR). Before generation, freeze monthly inputs with effective month and source; validate required fields at configuration/slice boundaries. Editing an already closed month creates a branch from its snapshot, starts a new trial as defined in ADR-15, and records changed inputs/effective month; never rewrite a committed branch silently.

## Alternatives considered
- AND-combined stopping conditions: can continue after a configured safety/desired stop is met.
- Retroactively edit a committed month: destroys audit history and reproducibility.

## Consequences
Run control, configuration, and monthly input snapshots are separate explicit records. Mid-month changes are not supported.
