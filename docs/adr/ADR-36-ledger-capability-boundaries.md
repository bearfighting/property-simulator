# ADR-36: Ledger capability boundaries

- **Status:** Superseded in its transaction/posting flow by [ADR-37](ADR-37-slice-journal-entry-contract.md); account, ledger, and reporting responsibility boundaries remain useful context
- **Date:** 2026-10-23
- **Related:** [Domain model](../domain-model.md), [Architecture](../architecture.md), [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md), [ADR-35](ADR-35-ordinal-month-periods.md)
- **Verification examples:** Confirm Application supplies the account set and Month; posting validates a candidate without calling Ledger; Ledger appends only accepted transactions; reports derive values from opening facts and journal entries.

## Context

The broad `accounting` label mixed account definitions, transaction data, posting rules, the Ledger journal, and financial reports. Application owns simulation use-case coordination and selects the account set at initialization. Ledger should record supplied facts and accepted transactions rather than create accounts or advance simulation time.

## Decision

- Use the capability names `account`, `transaction`, `posting`, `ledger`, and `reporting` for the current design direction; do not create a broad `accounting` crate by default.
- `account` defines account values and their accounting meaning, including account class and debit/credit balance behavior. Application selects and supplies the concrete account set at initialization. An optional opaque `AccountRole` key may carry configured business/reporting meaning; reporting explicitly maps roles it supports and does not parse names or account codes.
- `transaction` defines candidate transaction and entry values, including `Month`, account references, money, and provenance.
- `posting` validates candidate transactions against the supplied account definitions and double-entry rules. It produces a value that Ledger may append and does not call or mutate Ledger.
- `ledger` receives account definitions and opening facts at initialization, then appends accepted transactions to an append-only journal. It does not create accounts or Month values. Balances are derived from opening facts and journal entries.
- `reporting` derives balances, statements, cash flow, and audit checks from account definitions, opening facts, and posted journal data; it does not maintain competing accounting totals.
- Crate dependencies may carry these value types in one direction. The separation is about responsibility and control flow; it does not require a dependency-free graph. Do not introduce cycles or make Ledger depend on posting behavior.
- A journal position may locate an appended transaction; no generic transaction ID is required by this decision.

## Alternatives

- A single broad `accounting` crate: not selected as the default because it obscures distinct data and behavior responsibilities.
- Let Ledger create accounts or Month values: rejected because Application owns initial assembly and simulation progression.
- Let posting append directly to Ledger: not selected because Application should coordinate validation and append, keeping posting independent of Ledger behavior.
- Make reporting own balances or cash-flow totals: rejected because reports must derive from the journal and opening facts.

## Consequences

- Application initialization supplies account definitions and opening Ledger facts; monthly flow supplies Month values and coordinates `posting` followed by Ledger append.
- Slice outputs remain candidate transactions until posting accepts them.
- Concrete account-reference, accepted-transaction, journal-position, and report-input types remain to be designed before crate implementation.
