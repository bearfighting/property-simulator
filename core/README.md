# New Core Design

Core crates are members of the repository-root Cargo workspace, while their source files are grouped under this directory by responsibility. This keeps Core independently bounded from future Application and Presentation workspaces and lets Cargo tools discover active Core crates from the repository root. The frozen source crates under the repository's `crates/` directory are retained as references for domain behavior, not as active workspace members or templates for this structure. The `accounting/`, `foundation/`, and `simulation/` folders are organizational directories, not nested workspaces.

## Interface design work

Agree on module ownership, dependency direction, and the interfaces between Core modules before adding domain implementations or migrating the existing workspace.

1. Keep each Slice's typed parameters and inputs in its Core contract. Application owns user configuration collections and validates updates; Core does not define a whole-settings aggregate.
2. Define `Month(0)` as the simulation initialization boundary and `Month(1)` as its first calculated month. Defer generic ID types until a concrete business reference requires them.
3. Define `account` values and the account collection selected by Application at initialization.
4. Define `journal-entry` as the balanced, Month-indexed accounting value. Initialization records Month 0 facts; Slice calculations return entries for Month 1 or later. Its constructor validates positive rounded lines and debit/credit equality.
5. `ledger` receives the account catalogue when validating or projecting entries, retains the currency and ordered opening/monthly `JournalEntry` facts, and derives ending balances from those entries.
6. `reporting` accepts a read-only account catalogue, currency, and ordered journal data and derives trial balance, balance sheet, period income/expense, raw cash-account movements, and audit counts. The Month 0 initialization entry establishes starting balances; period reports begin at Month 1.
7. Keep `SliceResult` scoped to one Slice and one Month. Application and Data Access organize events and journal entries by Month; Core does not define a cross-Slice Snapshot aggregate or generic Effect continuation record.
8. Keep Slice constructors and evaluation methods typed to each behavior. Deterministic Slices implement `DeterministicSlice::calculate`; stochastic Slices implement `StochasticSlice::simulate` with an explicit random source. Every Slice returns the shared `SliceResult` value to Application without a universal Core dispatcher.
9. Define which basic mathematical algorithms are genuinely shared and keep Slice-specific parameter derivation with its owning Slice.
10. Review the Application-facing Core calls only after these module contracts and their dependency directions are stable.

Use `cargo check --workspace` from the repository root to check active Core crates. Existing code under `crates/` is domain-behavior reference material only.

Stop-condition values and pure month-boundary evaluation live in the dedicated `stop-condition` crate; the caller supplies completed-month and loan-payoff facts.
