# ADR-07: Monthly cash-flow posting

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-13](ADR-13-ledger-profile.md)
- **Test vectors:** [Ledger vectors](../test-vectors.md#ledger-vectors)

## Context
The initial monthly model has no receivables or payables timing subsystem.

## Decision
Represent each cash-flow item through its source-linked Ledger postings and cash-flow classifications, and post modeled accrual and cash settlement in the same monthly close. Preserve Ledger detail; derive cash-flow amounts and monthly aggregates from those postings. Do not create a separate authored cash-flow record alongside the Ledger.

## Alternatives considered
- Separate accrual and settlement periods: would require explicit receivable/payable state not in initial scope.

## Consequences
This is a simulation simplification, not a real-world accounting or tax conclusion.
