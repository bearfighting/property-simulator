# ADR-61: Asynchronous SQLite Data Access adapter

- **Status:** Accepted
- **Date:** 2026-10-24
- **Related:** [ADR-29](ADR-29-layered-calculation-and-application-boundaries.md), [Data Access contract](../../data-access/README.md), [SQLite schema](../../data-access/sqlite/README.md)
- **Verification:** The SQLite adapter accepts an externally created pool, maps DTOs to the schema, and commits each accounting month, its events, and journal entries in one transaction.

## Context

Core calculations are synchronous, while database work belongs at the
Application/Data Access boundary. The Data Access DTOs are storage-neutral;
SQLite encoding and row layout must remain within a backend adapter. The host
application also owns database initialization and resource lifetime.

## Decision

- Implement SQLite persistence in a separate `data-access/sqlite` crate that
  depends on the storage-neutral `data-access` DTO crate.
- Use SQLx 0.8 with Tokio for asynchronous SQLite I/O. Keep all async calls in
  the adapter boundary; Core calculations remain synchronous.
- The host composition root creates the `SqlitePool`, using adapter-provided
  connection options that enable foreign keys, and injects the pool into
  `SqliteDataAccess`.
- Keep row mapping and SQL statements in the adapter. DTOs do not contain SQL
  field encodings or SQLite row identifiers.
- Save a Month boundary, its Events, and JournalEntries atomically. Effect
  persistence remains deferred until its representation is decided.
- Install the current schema with idempotent table creation. This does not add
  a schema versioning or migration policy.

## Alternatives considered

- Synchronous `rusqlite`: not selected because the Application/Data Access
  boundary is intended to support asynchronous database access.
- Put SQLx types and queries in the generic Data Access crate: rejected because
  backend details would leak into its storage-neutral contract.
- Let the SQLite adapter open its own database file: rejected because the host
  composition root owns resource creation and injection.

## Consequences

- SQLite access methods are async and require the host's Tokio runtime.
- The adapter can be replaced without changing DTOs; another backend still
  needs its own mapping and store implementation.
- Application-facing persistence ports remain to be aligned with the final
  Application use cases.
- Until Effect persistence is added, the month operation does not yet retain all
  continuation state required for complete simulation resumption.
