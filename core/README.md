# New Core Design

Core crates are members of the repository-root Cargo workspace, while their source files are grouped under this directory by responsibility. This keeps Core independently bounded from future Application and Presentation workspaces and lets Cargo tools discover active Core crates from the repository root. The frozen source crates under the repository's `crates/` directory are retained as references for domain behavior, not as active workspace members or templates for this structure. The `accounting/`, `foundation/`, and `simulation/` folders are organizational directories, not nested workspaces.

## Interface design work

Agree on module ownership, dependency direction, and the interfaces between Core modules before adding domain implementations or migrating the existing workspace.

1. Fill in the remaining Slice parameter structs and whole-settings validation rules in `slice-catalogue`/`context`; keep Context settings and Slice creation contracts from creating a `context` ↔ `slice` crate dependency cycle.
2. Define the `Month` value crate as a zero-based ordinal within one simulation, with `Month(0)` as its first calculated period. Defer generic ID types until a concrete business reference requires them.
3. Define `account` values and the account collection selected by Application at initialization.
4. Define `journal-entry` as the balanced, month-scoped accounting value returned by Slice calculations. Its constructor validates positive rounded lines and debit/credit equality.
5. `ledger` now retains account definitions and opening facts, appends `JournalEntry` values atomically in month order, and exposes balances derived from the journal and opening facts.
6. `reporting` now accepts read-only account, opening-balance, and journal data and derives trial balance, balance sheet, period income/expense, raw cash-account movements, and audit counts. Operating/investing/financing categories await explicit account-role and source semantics.
7. Complete Snapshot's event and journal result fields after Ledger's interface is defined. Keep each month's events in that month's Snapshot; do not model historical events as an active/inactive queue.
8. Keep Slice constructors and evaluation methods typed to each behavior. Deterministic Slices implement `DeterministicSlice::calculate`; stochastic Slices implement `StochasticSlice::simulate` with an explicit random source. Every Slice returns the shared `SliceResult` value to Application without a universal Core dispatcher.
9. Define which basic mathematical algorithms are genuinely shared and keep Slice-specific parameter derivation with its owning Slice.
10. Review the Application-facing Core calls only after these module contracts and their dependency directions are stable.

Use `cargo check --workspace` from the repository root to check active Core crates. Existing code under `crates/` is domain-behavior reference material only.
