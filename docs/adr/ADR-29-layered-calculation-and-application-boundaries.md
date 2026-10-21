# ADR-29: Application and data-access boundaries

- **Status:** Accepted; storage details remain open
- **Date:** 2026-10-21
- **Scope:** Application orchestration and persistence boundary

## Context

The simulation needs to coordinate user input, single-month calculation, and persistence while keeping domain calculations independent of UI and storage. The application must also support continuing a simulation across separate calls and reading saved results.

## Decision

Use the dependency direction `Presentation → Application → Data Access` and `Application → single-month calculation → Core`. Application is the only coordinator and depends on a narrow storage interface, not SQLite. It exposes only user-facing use cases; it does not expose database operations or calculation internals. Code identifiers do not use implementation-stage names or numbers.

## Alternatives

- Let Presentation call storage or calculation directly: rejected because it would split orchestration across layers.
- Let the monthly calculation perform IO or own the multi-month loop: rejected because it is a synchronous calculation boundary.
- Store authored cash-flow facts separately from Ledger transactions: rejected because cash flow is derived from Ledger facts.

## Consequences

Application loads the persisted values needed for each use case, performs the work in memory for that call, and persists accepted input changes or completed monthly output. It does not require resident simulation state. The application version is the only software version identifier; schema migration and cross-upgrade compatibility are outside scope. Failed-run recovery, replay guarantees, branches, multiple scenarios, and parallel scheduling are outside scope.

The current use cases, persistence behavior, domain state semantics, and remaining design tasks are maintained in the [roadmap](../implementation-roadmap.md), [architecture](../architecture.md), and [domain model](../domain-model.md). This ADR records the boundary decision and its rationale; those documents own operational details.
