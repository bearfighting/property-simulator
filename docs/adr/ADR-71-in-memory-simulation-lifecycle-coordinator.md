# ADR-71: In-memory simulation lifecycle coordinator

- **Status:** Accepted
- **Date:** 2026-10-26
- **Related:** [Architecture](../architecture.md), [ADR-60](ADR-60-core-simulation-contracts-without-aggregates.md), [ADR-70](ADR-70-immutable-simulation-configuration.md)
- **Verification:** `simulation-lifecycle/tests/lifecycle.rs` exercises allowed work units and legal transitions.

## Context

One simulation is a single run from initialization through its final report.
The Core defines calculations and stop-condition evaluation, while Application
exposes initialization, monthly simulation, and reporting work units. Neither
layer should own the UI-driven lifecycle coordination policy.

## Decision

- Define `SimulationState` (`Initial`, `Running`, `Reporting`) and the three
  authorized `WorkUnit` values in a small standalone `simulation-lifecycle`
  crate, independent of Core, Application, Presentation, and storage.
- The composition layer holds the current `SimulationState` in memory and
  selects the associated work unit.
- `Initial` corresponds to initial settings, `Running` to monthly simulation,
  and `Reporting` to report generation. The state transition is linear:
  `Initial` → `Running` → `Reporting`.
- Monthly simulation may be invoked repeatedly while the current state is
  `Running`; the composition layer decides when to advance to `Reporting`.
- Lifecycle state is not persisted. Resume and recovery are outside scope.

## Alternatives considered

- Put lifecycle state in Core: rejected because it coordinates application
  work units rather than defining a calculation or individual domain rule.
- Put the state machine in Application: rejected because Application remains
  three independent work units and does not own UI-driven process lifetime.
- Put transition policy directly in UI handlers: rejected because it would
  make lifecycle rules depend on presentation code and harder to test alone.

## Consequences

- Desktop or another composition layer can select the corresponding work unit
  without changing Application's public work-unit boundaries.
- The lifecycle describes the sequence; it does not validate or reject calls.
- A process restart starts a new run; resuming a previous run is not supported.
