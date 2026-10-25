# ADR-59: Application-owned simulation random root

- **Status:** Accepted
- **Date:** 2026-10-24
- **Supersedes:** The Context ownership decision in [ADR-34](ADR-34-simulation-random-root-lifecycle.md); related root placement in [ADR-39](ADR-39-context-settings-and-month-zero-start.md) and [ADR-42](ADR-42-user-settings-and-opening-values.md)
- **Related:** [Random engine](../../core/simulation/random-engine/src/lib.rs), [SQLite schema](../../data-access/sqlite/schema.sql)
- **Verification:** Application obtains one root at simulation initialization, persists it independently of settings, and passes it to random-stream derivation when needed.

## Context

The Core `Context` aggregate is being removed. The simulation random root still
needs to live for the simulation's lifetime and be persisted, but it is not a
user setting and does not need to be grouped with settings or monthly results.

## Decision

- Application obtains one random root from the system when initializing a
  simulation and retains it as simulation state.
- Data Access persists the root independently of Slice settings and monthly
  records. Once initialized, the root is immutable for that simulation. Core
  does not own or persist the root.
- Keep `RandomRootKey` and stream derivation in `random-engine`. The caller
  passes the root and an explicit semantic identity to derive a stream; Slices
  receive only the random input they need.
- Do not persist evolving stream cursors or duplicate random stream state in
  per-month results when inputs are derived from the root and identity.

## Alternatives considered

- Store the root in Core `Context`: rejected because Context is an aggregate
  being removed from Core and the root has an independent lifecycle.
- Treat the root as a user setting or attach it to each month's output:
  rejected because it is system-provided, simulation-scoped state.

## Consequences

- Removing Core Context does not change the random-engine API or Slice random
  input interfaces.
- Application/Data Access owns root generation, retention, and persistence.
- The random root remains a separate simulation-level stored value.
