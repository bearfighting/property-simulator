# ADR-38: Reporting projections

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [Domain model](../domain-model.md), [Architecture](../architecture.md), [ADR-37](ADR-37-slice-journal-entry-contract.md)
- **Verification examples:** Reporting derives a trial balance as of a Month, period income/expenses for inclusive month endpoints, and categorized cash flows from supplied account classifications; each entry's category amounts reconcile to its cash-account movements.

## Context

Reporting needs to derive views from account definitions, opening facts, and the journal. It should not own Ledger state or create a reverse dependency from Ledger to Reporting. The higher layer owns account selection and cash-flow category semantics; Reporting applies classifications supplied with the account set.

## Decision

- Reporting functions accept read-only account catalogue, Ledger currency, opening-balance slice, and journal-entry slice as explicit input. Reporting does not accept or mutate a `Ledger` object and Ledger does not call Reporting.
- The projections are trial balance and balance sheet through an inclusive Month, income/expense activity over an inclusive Month interval, categorized cash-flow movements, and an audit validation/count result.
- The caller supplies cash account codes and a classification for each counterpart account: operating, investing, or financing plus a caller-defined subcategory. Reporting derives signed categorized movements from counterpart journal lines; debit counterparts are cash outflows and credit counterparts are cash inflows.
- Each classified movement carries Month, counterpart account, category, subcategory, and zero-based journal position. This position is a locator, not a persistent transaction ID. Category and subcategory totals are projections of these movements and must reconcile to selected cash-account movements for each journal entry.
- Reporting does not infer cash-flow classifications from account names, codes, or roles. It requires an explicit classification for every non-cash counterpart account in an entry that moves a selected cash account.
- Reporting depends on account, journal-entry, ledger opening-balance, money, and month value types. No reverse dependency is introduced.

## Alternatives

- Let Reporting call Ledger query methods: not selected; callers supply immutable ledger facts directly.
- Store cash-flow categories on every `JournalEntry`: rejected because they derive from counterpart account lines and the classification supplied with the account set; JournalEntry remains independent of reporting policy.
- Track each cash flow to a Slice activity instance: not required for the simulator's reporting goal; categorized Ledger movements and transaction locators provide the needed detail.

## Consequences

- Reporting provides general-ledger projections without becoming an accounting coordinator or deciding account semantics.
- The projections can be called from tests, Application, or other consumers using the same immutable inputs.
- The higher layer determines account classifications when it selects the account set and passes them to Reporting. No JournalEntry Note or activity-instance source field is needed for cash-flow categories.
