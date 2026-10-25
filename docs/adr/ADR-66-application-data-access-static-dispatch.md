# ADR-66: Application and Data Access static-dispatch boundary

- **Status:** Accepted
- **Date:** 2026-10-24
- **Related:** [ADR-29](ADR-29-layered-calculation-and-application-boundaries.md), [ADR-61](ADR-61-asynchronous-sqlite-data-access.md), [Architecture](../architecture.md), [Implementation roadmap](../implementation-roadmap.md)

## Context

Callers use asynchronous persistence while remaining independent of SQLite
and SQLx. The storage-neutral `data-access` crate owns DTOs and the `DataAccess`
trait; a concrete SQLite adapter implements that contract. The caller's
composition root creates the adapter and supplies it to the runtime layer.

## Decision

- Define storage-neutral store traits alongside DTOs, grouped by simulation
  facts, settings, and monthly facts. Provide a composed `DataAccess` trait for
  callers that need every group. Keep this crate independent of SQLite, SQLx,
  Application, and Presentation.
- Expose operations over the persisted facts represented by the schema:
  simulation identity/root/account/month data, per-Slice settings,
  stop-condition sets, monthly events, repair cost instances, and JournalEntries.
  Month-scoped event and JournalEntry reads are available alongside full-history reads.
  Do not expose table-level generic CRUD operations.
- Have `data-access/sqlite` depend on and implement the contract crate. The
  composition root creates the SQLx pool and adapter, then injects the adapter
  into `Application<D>` where `D` implements the persistence trait(s).
- Use generic static dispatch. Do not use `dyn Trait`, boxed futures, or an
  `async-trait` abstraction unless a later requirement needs runtime selection
  of implementations.
- Keep errors in the contract backend-neutral. Do not require `Send` on public
  async trait futures until a caller's executor or task-spawning boundary needs it.

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
- The current trait maps to the existing SQLite persistence operations. A
  future caller may motivate narrower trait bounds or a `Send` future contract;
  neither is required by the current persistence boundary.

## Verification examples

- `Application<SqliteDataAccess>` can be constructed by the composition root
  without the Application crate importing the SQLite crate.
- An in-memory adapter can implement the same persistence contract and be
  injected into `Application<MemoryDataAccess>`.
- The monthly persistence operation accepts the Month, events, journal
  entries, and generated repair activity lifecycle facts through the contract.
