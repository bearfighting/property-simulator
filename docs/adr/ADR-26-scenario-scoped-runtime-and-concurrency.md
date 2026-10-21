# ADR-26: Single-threaded calculation scope and deferred scenario management

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related:** [roadmap](../implementation-roadmap.md), [domain model](../domain-model.md), [architecture](../architecture.md), [desktop application](../desktop-application.md), [ADR-22](ADR-22-m3-run-inputs-snapshot-and-cashflow-contract.md), [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md)
- **Test vectors:** No concurrency or scenario-manager behavior is implemented in the current core.

## Context

The current work defines calculation modules, not a running application. CLI and GUI may later manage multiple simulation scenarios with independent settings. That higher-level workflow is not needed to implement the current core calculations.

## Decision

- Core calculation APIs are synchronous, stateless input/output calls. No run context, scenario registry, schedule, archive, or concurrency policy is implemented here.
- Treat the current project work as single-threaded. Do not add synchronization or parallel scenario execution to the calculation modules.
- Multiple scenarios, their independent settings, and any future scheduling are deferred to a higher application layer. This ADR does not define how that layer will implement them.

## Alternatives considered

- Add a core-wide scenario registry or parallel runner: rejected because the current calculation work has no scenario-management requirement.
- Design multi-scenario ownership now: deferred until the CLI/GUI workflow is specified.

## Consequences

The current core has no persistent scenario/run context and assumes single-threaded calls. Future CLI/GUI work can introduce scenario ownership and decide whether to schedule calls sequentially or concurrently without putting that policy into calculation modules.
