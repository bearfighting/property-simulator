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

Internal invariant violations, checked overflow during an unexpected calculation, and infrastructure failures are calculation errors, not domain outcomes and not user input errors. The calculation caller decides how to stop and report them. A failed function call yields no successful result. This ADR does not define a resident simulation run, snapshot lifecycle, retry/recovery behavior, or external persistence guarantees. The current production crate exposes independent calculation functions; older runtime behavior exists only in test support.

Calculation errors retain their typed cause and, where available, the activity context. Production Slice proposal evaluation reports the `SliceTransition` fatal phase and has no run identity. The other phase values, run identity fields, and transaction preflight diagnostic types exist only for test-support composition; they do not imply a production coordinator or run lifecycle. Validation failures remain their input/proposal categories. Unsupported Effect targets and transaction preflight failures (including account errors and transaction keys) are proposal validation errors in the test-support composition fixture.

## Alternatives considered
- Return one broad `DomainError`/`AccountError` from every calculation: rejected because it allows predictable invalid input to fail late and conflates user correction with implementation failure.
- Silently clamp, skip, or invent balancing activities after an error: rejected because it changes modeled economics and hides defects.
- Treat every unexpected runtime failure as invalid user input: rejected because it misattributes implementation or infrastructure faults.

## Consequences
Validation and computation APIs have distinct responsibilities. The ledger fixes its cash-position account set from chart metadata and validates cash-flow classification during transaction posting. `validate_projection()` checks range and configured cash-account selection and returns a `ValidatedProjection` borrowing the ledger; its `project()` method performs computation after the validation boundary. Arithmetic overflow remains an explicit checked numeric error at the documented finite boundary; it is never a user-correctable economic outcome.
