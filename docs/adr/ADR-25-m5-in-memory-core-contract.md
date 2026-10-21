# ADR-25: M5 in-memory core data contract

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related:** [roadmap](../implementation-roadmap.md), [domain model](../domain-model.md), [ADR-08](ADR-08-dataset-output.md), [ADR-10](ADR-10-delivery-phases.md), [ADR-19](ADR-19-validation-only-domain-errors.md)
- **Test vectors:** M5 runtime data invariants use the existing run-control and in-memory archive tests; no serialization vectors apply.

## Context

The earlier M5 outline combined stabilization of the core runtime data contract with serialization and persistence concerns. The project uses CLI and GUI inputs rather than Web/API schema-mapped inputs, and SQLite persistence is scheduled for Phase II. M5 should stabilize the runtime core without prematurely defining a storage representation or generic external-data validator.

## Decision

- M5 covers typed in-memory configuration, run control, completed results, Ledger state, snapshots, branch relationships, and latest user settings. It does not implement serialization, file I/O, SQLite, or database load validation.
- CLI and GUI own syntax parsing, input affordances, and user-facing feedback. The core API remains responsible for domain constraints and cross-field validation required for correct computation; presentation layers must not duplicate domain rules.
- Runtime constructors, archive operations, and composition boundaries enforce their own invariants. This is validation of core-owned typed data, not a general validator for external serialized datasets.
- JSON/CSV/XLSX representation, SQLite persistence, and any handling of incompatible local data are Phase II concerns. M5 does not define a general external serialization schema, cross-upgrade compatibility, or automatic migration. The application version is the sole software version identifier; model rules and random-stream behavior change with the application as a whole and have no independent version fields.

## Alternatives considered

- Define a stable serialized schema during M5: rejected because no persistence or external schema-mapped input boundary is being implemented in this milestone.
- Put all validation in CLI/GUI: rejected because the core API must preserve its domain invariants regardless of which presentation calls it.
- Add a generic external dataset validator before storage exists: rejected as speculative infrastructure; validate actual persistence boundaries when Phase II introduces them.

## Consequences

M5 acceptance focuses on complete in-memory runs, archive queries, snapshot branching, traceability, and domain invariants. Phase II may choose internal SQLite and export representations based on actual use. It must not assume unfinished-run recovery or cross-upgrade compatibility unless a later decision adds those requirements.
