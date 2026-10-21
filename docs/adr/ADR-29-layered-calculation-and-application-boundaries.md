# ADR-29: Layered calculation and application boundaries

- **Status:** Accepted
- **Date:** 2026-10-21
- **Related:** ADR-26, ADR-27, ADR-28
- **Test vectors:** Single-month composition vectors are assigned to M6; application read/calculate/persist flow is tested when that later layer is implemented.

## Context

The earlier roadmap assigned complete monthly input/output composition to M5, while the pre-M5 review concluded that current production modules should remain independently callable and should not contain premature system orchestration. The project needs a clear boundary between reusable calculations, assembling one month's calculation, user-facing application flow, and persistence.

The intended data flow is from the user interface to the application layer. The application reads user settings, opening state, and Ledger data through data access, passes those values as parameters into a single-month calculation, receives the complete monthly result, and asks data access to persist the user inputs, monthly result, and Ledger records. The result includes events, state changes, the closing-state snapshot, Ledger changes, and the per-instance cash-flow sequence derived from Ledger entries. The Ledger changes in the result and the Ledger records passed to data access are the same economic facts, not a second ledger. Cash flow is a result projection, not a competing hand-maintained accounting record. UI code must not call data access or assemble calculation modules directly. The physical storage arrangement is deferred to the data-access design.

## Decision

Use these responsibility layers:

1. **Core modules:** domain, accounting, models, and individual Slice calculations. Each module exposes explicit input/output calculations. Modules may share value types and narrow interfaces, but must not directly call another module's concrete implementation. Required collaborators are passed by the caller.
2. **Monthly integration (M6):** a synchronous, stateless calculation boundary for one month. It prepares the month, calls the independent calculations through their interfaces, integrates Slice outputs, settles Ledger effects, audits the result, and returns one complete monthly result containing the closing state. It does not perform IO, repeat months, own a run, or manage scenarios.
3. **Application:** the top-level use-case boundary. It obtains data from data access, passes values to monthly integration, receives outputs, and requests persistence through data access. It owns the sequencing between these operations.
4. **Data Access:** persistence implementation, expected to include SQLite later. It is called only by Application. Its internal storage representation is a project concern.
5. **Presentation:** CLI/GUI parses and constrains user-facing formats, displays results, and invokes Application. It does not access storage or call monthly integration directly.

These arrows describe runtime calls: `Presentation → Application`, then Application calls Data Access and Monthly integration; Monthly integration calls Core modules. Application calls Data Access through an injected narrow interface, and the concrete storage adapter implements that interface. Application does not depend on a concrete SQLite implementation. Data Access is never called from Core or Monthly integration.

M5 completes Core modules only. M6 completes single-month integration only. Application, data access, and presentation are subsequent roadmap stages. M5/M6 and the current application scope are single-threaded and do not manage multiple scenarios. If a later CLI/GUI workflow needs multiple scenarios or parallel scheduling, the higher-level application design can address it then.

Only the application version is a software version identifier. Do not add independent domain/model/random-stream/schema versioning, schema migration, or cross-upgrade compatibility commitments. A simulation advances through its months sequentially, and a completed month is not recalculated within that flow. Unfinished-run recovery, retry, checkpoint restoration, and branch management are not part of this design.

## Alternatives considered

- **Put monthly coordination into Core modules:** rejected because it makes otherwise independent calculation modules call or own one another's concrete implementations and obscures their responsibilities.
- **Put SQLite access into monthly integration:** rejected because calculation should receive values and return values; persistence is owned by the top application flow.
- **Let CLI/GUI call storage and calculation directly:** rejected because it duplicates sequencing and couples presentation to calculation and persistence details.
- **Add scenario schedulers, branching, or concurrency now:** deferred because no current application requirement needs these behaviors.

## Consequences

- M5 and M6 have distinct acceptance criteria; the full monthly result is not a M5 deliverable.
- M6 can be tested as a real calculation composition without IO. Application tests later can use in-memory data access to verify read/compute/persist ordering.
- Data access and presentation can be developed later without changing domain rules, provided they pass explicit typed values through the agreed boundaries.
- The roadmap no longer treats old M3/M4 run/archive fixtures as current production architecture.

## Test vectors

- M5: test each Core module independently, including domain mathematics, accounting invariants, sampling boundaries, and error outputs.
- M6: compose at least two independent Slices; verify source traceability, balanced Ledger changes, Ledger-derived cash flow, closing state consistency, conflict rejection, and no successful partial result on failure.
- Application/data access stage: with a fake in-memory data-access implementation, verify the application reads inputs, passes them as typed parameters, persists the returned monthly result and Ledger records, and does not bypass the monthly integration boundary.
