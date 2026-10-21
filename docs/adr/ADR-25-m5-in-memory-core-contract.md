# ADR-25: M5 in-memory core data contract

- **Status:** Superseded by [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md)
- **Date:** 2026-10-20
- **Related:** [roadmap](../implementation-roadmap.md), [domain model](../domain-model.md), [ADR-08](ADR-08-dataset-output.md), [ADR-10](ADR-10-delivery-phases.md), [ADR-19](ADR-19-validation-only-domain-errors.md), [ADR-26](ADR-26-scenario-scoped-runtime-and-concurrency.md)
- **Test vectors:** This earlier scope was superseded; use ADR-27's monthly transition vectors.

## Context

The earlier M5 outline combined definition of the core runtime data contract with serialization and persistence concerns. The project uses CLI and GUI inputs rather than Web/API schema-mapped inputs, and SQLite persistence is scheduled for Phase II. M5 should define the runtime core without prematurely defining a storage representation or generic external-data validator.

## Decision

- This ADR originally assigned completed history and branch relationships to a core-owned `SimulationArchive`. That ownership decision is superseded by ADR-27; the remaining validation and versioning decisions below continue to apply.
- CLI and GUI own syntax parsing, input affordances, and user-facing feedback. The core API remains responsible for domain constraints and cross-field validation required for correct computation; presentation layers must not duplicate domain rules.
- Runtime constructors and composition boundaries enforce their own invariants. This is validation of core-owned typed data, not a general validator for external serialized datasets.
- JSON/CSV/XLSX representation, SQLite persistence, and any handling of incompatible local data are Phase II concerns. M5 does not define a general external serialization schema, cross-upgrade compatibility, or automatic migration. The application version is the sole software version identifier; model rules and random-stream behavior change with the application as a whole and have no independent version fields.

## Alternatives considered

- Define a stable serialized schema during M5: rejected because no persistence or external schema-mapped input boundary is being implemented in this milestone.
- Put all validation in CLI/GUI: rejected because the core API must preserve its domain invariants regardless of which presentation calls it.
- Add a generic external dataset validator before storage exists: rejected as speculative infrastructure. When Phase II adds persistence, it will apply only the decoding and integrity checks needed to construct valid core runtime objects from that internal representation.

## Consequences

The original M5 archive acceptance scope is superseded by ADR-27. The decisions excluding general external-schema validation, schema versioning, automatic migration, unfinished-run recovery, and cross-upgrade compatibility remain in force.

The current calculation modules are synchronous and single-threaded. They do not own a run or scenario. Multiple scenarios, their independent settings, and any scheduling belong to a future application layer; this ADR does not design that layer. See [ADR-26](ADR-26-scenario-scoped-runtime-and-concurrency.md).
