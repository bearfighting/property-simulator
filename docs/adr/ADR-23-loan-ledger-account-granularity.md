# ADR-23: Loan Ledger account granularity

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related:** [ADR-14](ADR-14-mortgage-interest.md), [ADR-22](ADR-22-m3-run-inputs-snapshot-and-cashflow-contract.md), [domain model](../domain-model.md)
- **Test vectors:** M-01–M-12 and M3.6 loan Slice boundary in [test-vectors.md](../test-vectors.md)

## Context

The loan Slice must reconcile each continuation balance to posted Ledger balances. ADR-22 left the liability account layout open, while the M3.6 implementation requires deterministic reconciliation per instance and the shared interest expense category.

## Decision

Each active loan instance uses its own non-contra liability account. Loan interest expense is posted to a shared expense account. Every payment and principal balance update posts balanced entries that reference the instance's liability account; the instance continuation balance must equal that account's credit balance. A settled loan's liability account must have a zero balance. A caller may remove a loan activity only after its continuation is settled.

## Alternatives considered

- Use a shared liability account with instance allocations: rejected for the initial implementation because account-level reconciliation would not independently detect a mismatch in one loan's continuation.
- Use a separate interest expense account per loan: rejected because interest classification is shared and provenance already attributes each posting to its loan instance.

## Consequences

Loan configuration must bind each instance to a unique liability account and the shared interest expense account. The account chart has one additional liability account per loan instance. Existing opening liabilities remain opening facts; no historical disbursement is fabricated. Run updates that increase or reduce principal require a balanced financing transaction against the same instance account. A loan instance cannot be introduced into an already-running `SimulationRun` without an explicit opening state; that lifecycle is outside this decision's M3.6 scope.
