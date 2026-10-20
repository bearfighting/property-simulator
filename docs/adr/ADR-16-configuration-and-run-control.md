# ADR-16: Configuration and run controls

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [roadmap](../implementation-roadmap.md), [domain model](../domain-model.md), [ADR-01](ADR-01-monthly-run-control.md), [ADR-04](ADR-04-scenario-and-overrides.md), [ADR-15](ADR-15-random-streams-and-branches.md), [ADR-21](ADR-21-m3-run-randomness-and-persistence.md)
- **Test vectors:** M3 lifecycle/configuration tests cover these semantics.

## Context
Initial facts, run bounds, and changing monthly assumptions have different lifetimes and provenance.

## Decision
Scenario contains opening facts, future assumptions, configured activity instances, enabled calculation rules/Slices, and explicit versions. Higher-level run controls may contain an inclusive start month, an optional end month, zero or more month-end termination conditions, and between-period continue/stop control. Any configured end month or termination condition ends the run when reached/satisfied (OR). The user may also issue a stop command between completed periods, including before a configured boundary is reached; if there is no configured boundary/condition, the user must decide between periods whether to continue or stop. These controls govern whether the caller invokes another single-period transition and are not inputs to Slice/domain calculations. Evaluate configured termination conditions after the period result and closing snapshot. Before a transition, accept user changes to previously configured parameters/assumptions, current known state, and activity instances; record their effective period and source, then freeze current-period inputs. Changes take effect from the selected period and do not rewrite already committed periods. A current-state adjustment that changes an economically accounted balance must be represented by a state Effect and balanced Ledger transaction. Editing a previously closed period creates a branch from its snapshot, starts a new trial as defined in ADR-15, and records changed inputs/effective period. Validate required fields at configuration/rule boundaries.

## Alternatives considered
- AND-combined stopping conditions: can continue after a configured safety/desired stop is met.
- Retroactively edit a committed month: destroys audit history and reproducibility.

## Consequences
Run control, configuration, and monthly input snapshots are separate explicit records. Mid-month changes are not supported.
