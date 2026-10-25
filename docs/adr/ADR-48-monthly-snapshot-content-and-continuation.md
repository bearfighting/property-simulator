# ADR-48: Monthly Snapshot contents and continuation

- **Status:** Accepted, amended by [ADR-50](ADR-50-context-is-source-of-current-settings.md) and [ADR-51](ADR-51-slice-owned-continuation-state.md)
- **Date:** 2026-10-23
- **Related:** [ADR-35](ADR-35-ordinal-month-periods.md), [ADR-37](ADR-37-slice-journal-entry-contract.md), [ADR-46](ADR-46-explicit-slice-construction-and-application-composition.md), [Domain model](../domain-model.md)

## Context

Each period creates new Slice instances from current inputs and prior completed state. Continuing a simulation requires a monthly record of Slice outputs and continuation state. Context remains the source of the current user settings.

## Decision

- A completed `Snapshot` records its `Month` and each `SliceResult` with its `SliceKind`. Each result retains its events, effects, and that Slice's committed `JournalEntry` values together. Snapshot does not copy `UserSettings`; Application selects the effective settings from Context before constructing Slice instances.
- Raw random samples are evaluation inputs, not persisted Snapshot data. If a sampled outcome changes domain state needed by later calculations, the Slice records that state change as a typed `Effect`.
- Snapshot stores the month's journal delta, not a copy of the whole Ledger or its derived account balances. Ledger remains the source for current balances.
- Slice-specific continuation state remains in the relevant Slice result as typed effects; Snapshot does not maintain a separate continuation projection for a particular Slice.
- Snapshot construction validates that every included `JournalEntry` belongs to the Snapshot month.
- Add continuation types as concrete Slice behaviors require them; do not persist raw random samples as Snapshot fields.

## Alternatives considered

- Store only Month and events: rejected because Slice results, accounting changes, and non-Ledger continuation state would be lost.
- Store the complete Ledger in every Snapshot: rejected because balances are derived from the Opening JournalEntry and committed monthly journal entries.
- Derive Slice state from Ledger: rejected when that state is not represented by accounting balances; the relevant Slice result carries its typed state change.

## Consequences

- Application gathers independent Slice results, appends their journal entries to a candidate Ledger, and constructs Snapshot from the completed results after the month succeeds.
- Context supplies current user settings; Ledger supplies current account balances; prior Slice results provide typed state changes for subsequent calculations.
- Snapshot is a monthly result record rather than an alternate accounting ledger.
