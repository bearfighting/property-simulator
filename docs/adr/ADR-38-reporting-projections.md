# ADR-38: Reporting projections

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [Domain model](../domain-model.md), [Architecture](../architecture.md), [ADR-37](ADR-37-slice-journal-entry-contract.md)
- **Verification examples:** Reporting derives a trial balance as of a Month, period income/expenses for inclusive month endpoints, and raw cash-account movements from supplied facts; invalid references, currencies, unbalanced entries, or arithmetic overflow return errors.

## Context

Reporting needs to derive views from account definitions, opening facts, and the journal. It should not own Ledger state or create a reverse dependency from Ledger to Reporting. The current JournalEntry has no source or cash-flow classification metadata, so some desired projections cannot yet be derived faithfully.

## Decision

- Reporting functions accept read-only account catalogue, Ledger currency, opening-balance slice, and journal-entry slice as explicit input. Reporting does not accept or mutate a `Ledger` object and Ledger does not call Reporting.
- The initial projections are trial balance and balance sheet through an inclusive Month, income/expense activity over an inclusive Month interval, raw cash-account movement detail when the caller supplies cash account codes, and an audit validation/count result.
- Cash movements use debit-positive/credit-negative signs and carry Month, account code, and zero-based position within the supplied journal slice. This position is a locator, not a persistent transaction ID.
- Operating/investing/financing classification and per-activity source attribution remain open until account-role semantics and JournalEntry provenance are designed. Reporting does not infer these from account name or code.
- Reporting depends on account, journal-entry, ledger opening-balance, money, and month value types. No reverse dependency is introduced.

## Alternatives

- Let Reporting call Ledger query methods: not selected; callers supply immutable ledger facts directly.
- Infer cash-flow categories from account names/codes: rejected because those are not stable machine semantics.
- Return categorized cash flows before account-role and source rules exist: deferred to avoid reporting data the model cannot justify.

## Consequences

- The initial Reporting crate provides useful general-ledger projections without becoming an accounting coordinator.
- The projections can be called from tests, Application, or other consumers using the same immutable inputs.
- Cash-flow category totals and activity-level audit detail require later domain metadata; raw cash movements alone do not satisfy the full traceability goal.
