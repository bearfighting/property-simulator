# ADR-62: Store month-scoped event facts without amounts

- **Status:** Accepted
- **Date:** 2026-10-24
- **Related:** [Domain model](../domain-model.md), [ADR-37](ADR-37-slice-journal-entry-contract.md), [SQLite schema](../../data-access/sqlite/README.md)
- **Verification:** Core Events are payload-free kinds; month event DTOs preserve repeated occurrences; SQLite stores event kind and month only; displayed amounts are derived from Ledger.

## Context

Event amounts duplicated values already recorded by JournalEntries. Users
consume monthly results and categorized cash flows, not JournalEntry rows. An
Event still records facts that do not necessarily create a JournalEntry, such
as a repair occurrence or a zero-cost repair settlement.

## Decision

- Represent Events as payload-free event kinds. The containing `SliceResult`
  supplies the Month; persisted event records are associated with that Month.
- Derive all displayed amounts from Ledger and reporting projections. Do not
  store or link an Event to a specific JournalEntry.
- Preserve repeated Event occurrences and their collection order within a
  Month using an adapter-level ordinal. It is only a storage position, not a
  business or Slice instance identifier.
- Do not persist the producing `SliceKind`; the event kind already identifies
  the event category, and no per-instance source is required.

## Alternatives considered

- Keep monetary payloads on Events: rejected because JournalEntries are the
  source of posted amounts and user-facing cash flow is Ledger-derived.
- Add an Event-to-JournalEntry relation: rejected because user-facing results
  need monthly/category amounts, not transaction-level provenance.
- Persist Slice instance identifiers: rejected because instance identity is
  not required for the agreed user-facing results.

## Consequences

- Event and Ledger facts remain separate: Events record occurrence; Ledger
  records monetary postings.
- The SQLite event table stores event kind, Month, and a month-local ordinal.
- A future query that needs a specific Event-to-entry correlation would require
  a new explicit decision and data relationship.
