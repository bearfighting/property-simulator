# ADR-55: Ledger journal as the source of balances

- **Status:** Accepted
- **Date:** 2026-10-24
- **Supersedes:** The separate opening-balance input decision in [ADR-37](ADR-37-slice-journal-entry-contract.md)
- **Related:** [Domain model](../domain-model.md), [ADR-38](ADR-38-reporting-projections.md), [ADR-48](ADR-48-monthly-snapshot-content-and-continuation.md)
- **Verification examples:** One or more Month 0 entries establish starting balances; ending balances recompute from the ordered journal; Month 0 contributes no period cash flow.

## Context

Ledger previously accepted independent per-account `OpeningBalance` values, retained them beside monthly JournalEntry values, and cached per-account net balances. This made an opening balance look like a long-lived accounting fact even though it is a balance projection at one time boundary. The account catalogue is also selected by Application and should be injected where Ledger needs account validation rather than owned as Ledger state.

## Decision

- `Month(0)` is the simulation initialization boundary. Opening facts supplied for initialization are mapped to one or more balanced `JournalEntry` values at Month 0. Their combined effect establishes starting balances; the choice of one or several entries is an Application-level organization decision. These are accounting initialization records, not historical economic events or a completed simulation month.
- Month 1 is the first calculated period. Snapshots accept only completed months starting at Month 1 and entries for their own month. Ledger accepts entries in nondecreasing Month order.
- Ledger retains its currency and ordered JournalEntry sequence. AccountCatalogue is injected when appending or deriving account-indexed balances; Ledger does not own the catalogue.
- Per-account balances, including the Month 0 opening projection and ending balances, are recomputed from JournalEntry values. Reporting derives balances and statements from the same ordered journal and injected account catalogue. Period cash-flow and income/expense ranges start at Month 1, excluding Month 0 initialization entries.
- No separate OpeningBalance entity, balance cache, or competing balance facts are maintained by Ledger.

## Alternatives

- Keep independent OpeningBalance values beside monthly JournalEntry values: rejected because the same Ledger would have two kinds of authoritative accounting facts and balances would require combining both.
- Treat opening facts as a separate period before Month 0: rejected because Month 0 already represents the initialization boundary and provides a unified time index for opening state and subsequent calculated months.
- Store AccountCatalogue inside Ledger: rejected because the catalogue is Application-selected configuration, not Ledger accounting state.

## Consequences

- Initialization must construct balanced Month 0 JournalEntry values from initial values and Application-provided account-role mappings before Month 1 calculation begins. One or multiple entries are valid and produce the same opening balance projection when their combined lines are equivalent.
- `Month` consistently identifies the initialization boundary and each elapsed simulated month; JournalEntry uses Month directly, without a second period enum.
- Persisted journal facts are sufficient to reconstruct Ledger balances; account definitions remain program configuration.
- `ending_balances` requires the matching account catalogue and derives every configured account's signed net balance from the journal.
