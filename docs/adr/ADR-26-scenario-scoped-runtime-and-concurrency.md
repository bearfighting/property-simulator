# ADR-26: Scenario-scoped runtime state and application-owned concurrency

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related:** [roadmap](../implementation-roadmap.md), [domain model](../domain-model.md), [architecture](../architecture.md), [desktop application](../desktop-application.md), [ADR-22](ADR-22-m3-run-inputs-snapshot-and-cashflow-contract.md), [ADR-25](ADR-25-m5-in-memory-core-contract.md)
- **Test vectors:** No new domain vectors; scenario scoping and archive invariants are covered by M5 runtime/archive tests.

## Context

The core needs a clear owner for editable settings and completed run history without introducing a multi-scenario manager or parallel scheduler. CLI and GUI may later operate on several scenarios at once, and each scenario needs independent settings and run history.

## Decision

- An in-memory `SimulationArchive` represents one scenario's run family: its runs, snapshots, branches, queries, and latest editable user settings. Latest settings are scoped to that archive and do not replace the frozen inputs stored with completed results or snapshots.
- The core executes each run as an ordered sequence of monthly transitions. It does not manage a collection of scenarios, schedule multiple scenario runs, or promise parallel execution.
- The application layer owns scenario collections and any concurrency policy. If it runs multiple scenarios concurrently, each uses independent scenario configuration, run-control state, and archive; there is no shared mutable global settings state in the core contract.
- This decision does not restrict the application process to one operating-system thread. It leaves multi-scenario concurrency and scheduling outside the current core milestone. Slice-result parallel evaluation remains governed separately by ADR-20.

## Alternatives considered

- Add a core-wide scenario registry and parallel runner: rejected because no current domain operation requires cross-scenario coordination, and scheduling belongs to the application layer.
- Keep latest settings global across all scenarios: rejected because settings are scenario-specific and must not silently affect another scenario or a saved snapshot.
- Require the whole application to be single-threaded: rejected because this is an unnecessary process-level restriction; the core only declines to define concurrency policy.

## Consequences

M5 can implement and validate one scenario's complete in-memory runtime contract. Future CLI/GUI work can compose independent scenario contexts and choose sequential or concurrent execution without changing domain semantics. No cross-scenario synchronization or shared-setting semantics are introduced into the core.
