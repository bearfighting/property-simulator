# ADR-66: Application and Data Access static-dispatch boundary

- **Status:** Accepted
- **Date:** 2026-10-24
- **Related:** [ADR-29](ADR-29-layered-calculation-and-application-boundaries.md), [ADR-61](ADR-61-asynchronous-sqlite-data-access.md), [Architecture](../architecture.md), [Implementation roadmap](../implementation-roadmap.md)

## Context

Application must use asynchronous persistence while remaining independent of
SQLite and SQLx. A concrete SQLite adapter should be created by the program's
composition root and supplied to Application. The current storage-neutral
`data-access` crate already owns DTOs but does not yet define Application-facing
persistence traits.

## Decision

- Define Application-facing persistence traits in the storage-neutral
  `data-access` crate alongside DTOs. Keep this crate independent of SQLite,
  SQLx, Application, and Presentation.
- Express persistence in stable semantic operations over DTOs, such as loading
  simulation facts, accepting effective settings, and saving a completed
  Month atomically with its Events, JournalEntries, and activity lifecycle
  changes. Do not expose table-level generic CRUD operations.
- Keep the concrete methods and any trait split aligned with Application
  use-cases; this ADR fixes the boundary and style, not the final method list.
- Have `data-access/sqlite` depend on and implement the contract crate. The
  composition root creates the SQLx pool and adapter, then injects the adapter
  into `Application<D>` where `D` implements the persistence trait(s).
- Use generic static dispatch. Do not use `dyn Trait`, boxed futures, or an
  `async-trait` abstraction unless a later requirement needs runtime selection
  of implementations.
- Keep errors in the contract backend-neutral. Decide whether public async
  trait futures require `Send` when the Application executor and task-spawning
  boundary are designed.

## Alternatives considered

- Make Application depend on the SQLite adapter: rejected because it couples
  use-cases to one persistence implementation.
- Put generic table CRUD in the trait: rejected because it exposes storage
  shape rather than stable application persistence semantics.
- Use `Arc<dyn DataAccess>` with boxed async futures: deferred because current
  composition chooses a concrete adapter when constructing the application;
  dynamic backend replacement during runtime is not required.

## Consequences

- Application has a compile-time dependency on the contract crate, but no
  dependency on the SQLite adapter. The SQLite adapter and Application depend
  on the same contract; the composition root assembles them.
- Startup injection remains straightforward while generic dispatch avoids a
  vtable and per-call boxed-future allocation.
- Fake in-memory implementations can satisfy the same trait for Application
  tests without adding SQLite to Application.
- Exact trait grouping, method signatures, and Future `Send` bounds remain
  open until Application use-cases and executor requirements are specified.

## Verification examples

- `Application<SqliteDataAccess>` can be constructed by the composition root
  without the Application crate importing the SQLite crate.
- An in-memory adapter can implement the same persistence contract and be
  injected into `Application<MemoryDataAccess>`.
- The monthly persistence operation commits the Month, events, journal
  entries, and generated repair activity lifecycle changes atomically.
