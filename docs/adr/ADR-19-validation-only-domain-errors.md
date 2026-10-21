# ADR-19: Validation boundary and fatal runtime failures

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [architecture](../architecture.md), [ADR-05](ADR-05-slice-coordination.md), [ADR-18](ADR-18-sufficient-state-transition.md), [ADR-20](ADR-20-composable-slices-and-random-sampling.md)
- **Test vectors:** Validation tests retain correctable error categories; accepted-input runtime failures carry phase, month, available identity, and typed source diagnostics; successful paths produce a result.

## Context
The current M1 error enums combine invalid user input, predictable arithmetic limits, projection precondition failures, and internal computation failures. User-correctable issues must be found before computation. Errors that nevertheless occur during accepted execution indicate a fatal simulation failure, not a request for the user to repair the scenario.

## Decision
All user-correctable errors are discovered at explicit validation boundaries: scenario/rule validation, opening-state validation, monthly-input freezing, and proposal validation before commit. Validation reports actionable issues and only yields a validated input/state when required invariants hold.

The single-period transition, ledger balance recomputation, and financial projections do not return errors for missing references, classifications, or invalid ranges; those are validated before computation. Money has an intentionally finite `i128` representation. Inputs outside its supported scale/coefficient are rejected, and a checked operation that exceeds `i128` returns an explicit arithmetic overflow error. This is an accepted boundary for fantastically large, non-realistic values; arithmetic must never wrap or clamp.

Internal invariant violations, checked overflow during an unexpected runtime calculation, and infrastructure failures are fatal simulation failures, not domain outcomes and not user input errors. The run stops and exposes engineering diagnostics. A failed period produces no successful result or snapshot, and run control retains the last successfully committed in-memory state and Ledger while marking the run failed; that run cannot continue in place. Recovery/retry of the failed run and consistency of external persistence after such a failure are not promised. Successful periods still produce invariant-valid results. This ADR does not claim that physical resource exhaustion or implementation defects are impossible.

The simulation error contract represents fatal failures with a phase (`SliceTransition`, `LedgerCommit`, `ClosingStateDerivation`, or `MonthlyResultConstruction`), period, optional trial/branch and activity-instance identity, and the original typed simulation error. Slice proposal evaluation has no run identity, so its trial and branch fields are absent. Validation failures remain their existing input/proposal categories. Unsupported Effect targets and transaction preflight failures (including account errors and transaction keys) are proposal validation errors. Fatal diagnostics do not promise recovery/retry of the failed run or rollback of external persistence; run control preserves the last successful in-memory commit boundary.

## Alternatives considered
- Return one broad `DomainError`/`AccountError` from every calculation: rejected because it allows predictable invalid input to fail late and conflates user correction with implementation failure.
- Silently clamp, skip, or invent balancing activities after an error: rejected because it changes modeled economics and hides defects.
- Treat every unexpected runtime failure as invalid user input: rejected because it misattributes implementation or infrastructure faults.

## Consequences
Validation and computation APIs have distinct responsibilities. The ledger fixes its cash-position account set from chart metadata and validates cash-flow classification during transaction posting. `validate_projection()` checks range and configured cash-account selection and returns a `ValidatedProjection` borrowing the ledger; its `project()` method performs computation after the validation boundary. Arithmetic overflow remains an explicit checked numeric error at the documented finite boundary; if encountered during a run, it is surfaced as a fatal simulation failure with engineering diagnostics, never as a user-correctable economic outcome.
