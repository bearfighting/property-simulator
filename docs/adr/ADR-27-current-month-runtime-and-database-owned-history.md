# ADR-27: Current-month calculation boundary and Ledger-derived cash flow

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related:** [roadmap](../implementation-roadmap.md), [domain model](../domain-model.md), [architecture](../architecture.md), [ADR-05](ADR-05-slice-coordination.md), [ADR-16](ADR-16-configuration-and-run-control.md), [ADR-19](ADR-19-validation-only-domain-errors.md), [ADR-21](ADR-21-m3-run-randomness-and-persistence.md), [ADR-25](ADR-25-m5-in-memory-core-contract.md), [ADR-26](ADR-26-scenario-scoped-runtime-and-concurrency.md)
- **Test vectors:** M6 verifies monthly input/output and cash-flow projection from Ledger entries. Persistence and branching are outside this decision.

## Context

Earlier M5 and runtime documentation expanded the core into a history/archive and branching system, and described preserving a successful boundary after fatal failure. The current work only needs monthly calculation. Treating persistence, history, or branching as part of the calculation obscures the simple input/output boundary.

## Decision

- The monthly calculation is an input/output operation: opening state and that month's validated inputs/samples enter; a complete monthly result comes out. The result includes the month’s events, state changes, Ledger updates, cash-flow result, and closing state needed for the next month.
- Ledger transactions are the source of economic postings. Cash flow is derived from posted Ledger entries and their cash-flow categories. The per-instance monthly cash-flow sequence is grouped from Ledger source metadata and is included in the completed monthly result/snapshot data, not authored as a second accounting record.
- Core code does not own history, branches, database access, or persistence. This milestone does not design those concerns; they can be addressed separately when SQLite work begins.
- A calculation error returns no successful result for that call. The caller treats an unexpected fatal error as a failed simulation; the Core does not retain a run or define failure recovery, retry, checkpoint, or replay behavior. Correctable user input is rejected before calculation so it can be fixed and submitted again.

## Alternatives considered

- Keep every completed result in a core-owned in-memory archive: deferred because monthly calculation does not need historical storage.
- Store independently authored cash-flow contributions beside Ledger postings: rejected because cash-flow sequences are projections of Ledger entries and their source metadata.
- Add branch or database rules to the monthly calculation: deferred; they are not needed for the current input/output contract.
- Preserve/recover a failed run or replay its random stream: rejected because a fatal run stops and failed-run recovery is not a product requirement.

## Consequences

M6 focuses on the monthly input/output contract. Cash-flow output is derived from Ledger entries and grouped by source instance. M5 completes independent Core modules; a later Application layer coordinates calculation and data access. History, branches, SQLite, and persistence design remain outside M5/M6. An error yields no result for the failed calculation; no failed-run recovery workflow is defined. See ADR-29.
