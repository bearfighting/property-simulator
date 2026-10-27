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

- Define `SimulationState` (`Initial`, `Running`, `Stopped`) and the three
  authorized `WorkUnit` values in a small standalone `simulation-lifecycle`
  crate, independent of Core, Application, Presentation, and storage.
- The composition layer holds the current `SimulationState` in memory and
  selects the associated work unit.
- `Initial` corresponds to initial settings, `Running` to the whole active
  simulation run, and `Stopped` to a terminated run. The state transition is
  linear: `Initial` → `Running` → `Stopped`.
- The `MonthSimulation` work unit is one atomic month and may be invoked
  repeatedly while the state is `Running`. Successful months leave the run in
  `Running`; a manual stop, satisfied automatic stop condition, or fatal
  monthly failure advances it to `Stopped`.
- Reset abandons the current in-memory run and creates a fresh `Initial`
  lifecycle. It does not delete the prior simulation's persisted records; the
  current desktop workflow does not provide a way to reopen those records.
- `Reporting` is the work unit allowed in `Stopped`; it is not a lifecycle
  state. Fatal monthly failures do not have a separate state in the initial
  design.
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
- The composition layer applies the `Running` → `Stopped` transition after a
  stop request, an automatic stop condition, or a fatal monthly error.
- A process restart starts a new run; resuming a previous run is not supported.
- Reset clears the active session identity and page state so the next
  initialization starts a separate simulation.
