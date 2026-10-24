# ADR-37: Slice journal-entry contract

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [Domain model](../domain-model.md), [Architecture](../architecture.md), [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md), [ADR-35](ADR-35-ordinal-month-periods.md), [ADR-36](ADR-36-ledger-capability-boundaries.md)
- **Verification examples:** A balanced entry for the current Month can be constructed; a zero, negative, or unbalanced entry cannot; Ledger receives the completed entry without depending on Slice-specific data.

## Context

The simulation controls the business rules that generate its accounting results. Debit/credit balance and the mapping from Slice activities to accounts are domain invariants in this environment. A generic transaction proposal followed by a separate posting-validation capability would add a boundary that does not match the intended flow. Ledger still needs a stable accounting value independent of any particular Slice.

## Decision

- Add a focused `journal-entry` crate as a shared value contract between Slice calculations and Ledger.
- A Slice calculation returns zero or more `JournalEntry` values. Each entry contains its simulation `Month` and two or more `JournalLine` values; each line contains an account code, debit/credit side, and positive currency-aware amount.
- Entries contain only the month and balanced journal lines. Descriptive notes and provenance metadata are not part of this contract; add typed provenance only when a concrete reporting or audit requirement defines its semantics.
- Constructing a line rounds the amount half-up to the currency minor unit and rejects values that are not positive after rounding. Constructing an entry verifies debit and credit totals match, using checked money arithmetic.
- Journal-entry validation errors are owned by the `journal-entry` crate. Slice calls the constructors and returns construction failures through its own calculation error path.
- Ledger accepts completed `JournalEntry` values and independently validates ledger-specific constraints such as account membership and ledger currency. Ledger does not perform Slice-to-account mapping.
- A Ledger is initialized with an `AccountCatalogue`, its single `Currency`, and separate opening balances. An opening balance is a positive account amount and debit/credit side; zero balances are omitted, each account appears at most once, and the opening debit and credit totals must match. Opening facts are not journal entries.
- Ledger appends entries in nondecreasing `Month` order; multiple entries in one month are allowed. An append validates all lines and computes candidate balances before changing the journal, so a failed append leaves the Ledger unchanged.
- Ledger exposes signed net balances where positive means debit and negative means credit. The opening facts and journal remain the source from which those balances can be rebuilt.
- The type has no generic ID or Slice/activity source field. Reporting locates each categorized cash movement by its journal position; the simulator does not require activity-instance attribution for cash-flow reporting.
- `journal-entry` depends on the focused `account`, `money`, and `month` value crates. Slice and Ledger may depend on `journal-entry`; `journal-entry` does not depend on either.

## Alternatives

- Put the entry type in `ledger`: not selected because Slice would depend on Ledger just to produce its output.
- Keep separate `transaction` and `posting` crates: not selected for this controlled simulation because Slice calculation is responsible for producing a balanced result.
- Let Ledger receive Slice-specific result types: rejected because it would couple the stable journal to evolving Slice models.

## Consequences

- The append boundary is a validated accounting value rather than an arbitrary proposal.
- Ledger remains stable as Slice behavior evolves, provided each Slice can construct the shared value.
- Source provenance must be represented without introducing a dependency from the shared value crate to Slice-specific types.
- Initial balances remain opening facts, not fabricated historical journal entries. Their global accounting equation is validated as part of Ledger initialization, separately from `JournalEntry` construction.
