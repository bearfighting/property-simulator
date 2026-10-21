# ADR-25: M5 in-memory core data contract

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related:** [roadmap](../implementation-roadmap.md), [domain model](../domain-model.md), [ADR-08](ADR-08-dataset-output.md), [ADR-10](ADR-10-delivery-phases.md), [ADR-19](ADR-19-validation-only-domain-errors.md), [ADR-26](ADR-26-scenario-scoped-runtime-and-concurrency.md)
- **Test vectors:** M5 runtime data invariants use the existing run-control and in-memory archive tests; no serialization vectors apply.

## Context

The earlier M5 outline combined definition of the core runtime data contract with serialization and persistence concerns. The project uses CLI and GUI inputs rather than Web/API schema-mapped inputs, and SQLite persistence is scheduled for Phase II. M5 should define the runtime core without prematurely defining a storage representation or generic external-data validator.

## Decision

- M5 covers one scenario's typed in-memory configuration, run control, completed results, Ledger state, snapshots, branch relationships, and latest user settings. Each `SimulationArchive` owns the settings and run family for one scenario. It does not implement serialization, file I/O, SQLite, or a multi-scenario registry/scheduler.
- CLI and GUI own syntax parsing, input affordances, and user-facing feedback. The core API remains responsible for domain constraints and cross-field validation required for correct computation; presentation layers must not duplicate domain rules.
- Runtime constructors, archive operations, and composition boundaries enforce their own invariants. This is validation of core-owned typed data, not a general validator for external serialized datasets.
- JSON/CSV/XLSX representation, SQLite persistence, and any handling of incompatible local data are Phase II concerns. M5 does not define a general external serialization schema, cross-upgrade compatibility, or automatic migration. The application version is the sole software version identifier; model rules and random-stream behavior change with the application as a whole and have no independent version fields.

## Alternatives considered

- Define a stable serialized schema during M5: rejected because no persistence or external schema-mapped input boundary is being implemented in this milestone.
- Put all validation in CLI/GUI: rejected because the core API must preserve its domain invariants regardless of which presentation calls it.
- Add a generic external dataset validator before storage exists: rejected as speculative infrastructure. When Phase II adds persistence, it will apply only the decoding and integrity checks needed to construct valid core runtime objects from that internal representation.

## Consequences

M5 acceptance focuses on complete in-memory runs, archive queries, snapshot branching, traceability, and domain invariants. Phase II may choose internal SQLite and export representations based on actual use, adding only the checks needed to restore valid runtime objects. It does not imply a general external-schema validator, schema versioning, automatic migration, unfinished-run recovery, or cross-upgrade compatibility.

M5 treats each run as sequential monthly execution. Application-level management of multiple scenario archives and any concurrent execution policy are deferred; this does not constrain the application process to one thread. See [ADR-26](ADR-26-scenario-scoped-runtime-and-concurrency.md).
