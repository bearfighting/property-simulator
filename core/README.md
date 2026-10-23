# New Core Design

This is the standalone Cargo workspace for Core. It defines Core responsibilities and interfaces independently from Application and Presentation workspaces. Crates in the repository-root legacy workspace are references for domain behavior, not templates for this structure. This workspace currently contains exploratory crates grouped under `accounting/`, `foundation/`, and `simulation/`; the folders organize members of this one workspace and are not nested workspaces.

## Interface design work

Agree on module ownership, dependency direction, and the interfaces between Core modules before adding domain implementations or migrating the existing workspace.

1. Fill in the remaining Slice parameter structs and whole-settings validation rules in `slice-catalogue`/`context`; keep Context settings and Slice creation contracts from creating a `context` ↔ `slice` crate dependency cycle.
2. Define the `Month` value crate as a zero-based ordinal within one simulation, with `Month(0)` as its first calculated period. Defer generic ID types until a concrete business reference requires them.
3. Define `account` values and the account collection selected by Application at initialization.
4. Define `journal-entry` as the balanced, month-scoped accounting value returned by Slice calculations. Its constructor validates positive rounded lines and debit/credit equality.
5. `ledger` now retains account definitions and opening facts, appends `JournalEntry` values atomically in month order, and exposes balances derived from the journal and opening facts.
6. `reporting` now accepts read-only account, opening-balance, and journal data and derives trial balance, balance sheet, period income/expense, raw cash-account movements, and audit counts. Operating/investing/financing categories await explicit account-role and source semantics.
7. Complete Snapshot's event and journal result fields after Ledger's interface is defined. Keep each month's events in that month's Snapshot; do not model historical events as an active/inactive queue.
8. Finalize Slice creation and evaluation contracts. Each Slice kind owns the rule for creating zero or more instances; instances share one evaluation interface and return events and zero or more `JournalEntry` values.
9. Define which basic mathematical algorithms are genuinely shared and keep Slice-specific parameter derivation with its owning Slice.
10. Review the Application-facing Core calls only after these module contracts and their dependency directions are stable.

Do not migrate existing Core implementations into this workspace until the relevant contracts are agreed. Existing workspace code is domain-behavior reference material only.
