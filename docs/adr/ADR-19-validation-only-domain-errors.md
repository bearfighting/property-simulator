# ADR-19: Validation boundary and fatal runtime failures

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [architecture](../architecture.md), [ADR-05](ADR-05-slice-coordination.md), [ADR-18](ADR-18-sufficient-state-transition.md), [ADR-20](ADR-20-composable-slices-and-random-sampling.md)
- **Test vectors:** Validation tests reject malformed configuration, opening state, and frozen period input before transition; accepted-input tests assert every defined transition and projection produces a result.

## Context
The current M1 error enums combine invalid user input, predictable arithmetic limits, projection precondition failures, and internal computation failures. User-correctable issues must be found before computation. Errors that nevertheless occur during accepted execution indicate a fatal simulation failure, not a request for the user to repair the scenario.

## Decision
All user-correctable errors are discovered at explicit validation boundaries: scenario/rule validation, opening-state validation, monthly-input freezing, and proposal validation before commit. Validation reports actionable issues and only yields a validated input/state when required invariants hold.

The single-period transition, ledger balance recomputation, and financial projections do not return errors for missing references, classifications, or invalid ranges; those are validated before computation. Money has an intentionally finite `i128` representation. Inputs outside its supported scale/coefficient are rejected, and a checked operation that exceeds `i128` returns an explicit arithmetic overflow error. This is an accepted boundary for fantastically large, non-realistic values; arithmetic must never wrap or clamp.

Internal invariant violations, checked overflow during an unexpected runtime calculation, and infrastructure failures are fatal simulation failures, not domain outcomes and not user input errors. The run stops and exposes engineering diagnostics. The system makes no recovery, rollback, or persistent-consistency guarantee after such a failure; successful periods still produce invariant-valid results. This ADR does not claim that physical resource exhaustion or implementation defects are impossible.

## Alternatives considered
- Return one broad `DomainError`/`AccountError` from every calculation: rejected because it allows predictable invalid input to fail late and conflates user correction with implementation failure.
- Silently clamp, skip, or invent balancing activities after an error: rejected because it changes modeled economics and hides defects.
- Treat every unexpected runtime failure as invalid user input: rejected because it misattributes implementation or infrastructure faults.

## Consequences
Validation and computation APIs have distinct responsibilities. The ledger fixes its cash-position account set from chart metadata and validates cash-flow classification during transaction posting. `validate_projection()` checks range and configured cash-account selection and returns a `ValidatedProjection` borrowing the ledger; its `project()` method performs computation after the validation boundary. Arithmetic overflow remains an explicit checked numeric error at the documented finite boundary; if encountered during a run, it is surfaced as a fatal simulation failure with engineering diagnostics, never as a user-correctable economic outcome.
