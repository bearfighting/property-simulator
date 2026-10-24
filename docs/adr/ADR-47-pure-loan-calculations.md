# ADR-47: Keep loan arithmetic in pure domain calculations

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md), [ADR-46](ADR-46-explicit-slice-construction-and-application-composition.md), [Domain model](../domain-model.md)

## Context

The Loan Slice needs fixed-rate amortization arithmetic as well as behavior-specific validation and accounting output mapping. Keeping all formulas in the Slice implementation makes it harder to inspect the calculation separately from account mapping and event construction.

## Decision

- Put pure fixed-rate loan calculations in the `domain-calculations` crate. Its functions accept explicit numeric values and return calculated payment amounts without reading Slice settings, Context, Ledger, or IO.
- Keep Loan Slice responsible for validating whether the calculated installment is valid for the active loan state, selecting Application-provided account codes, and assembling events and `JournalEntry` values.
- Keep the calculation module loan-specific; do not create a general financial formula framework or move Slice configuration and lifecycle rules into it.

## Alternatives considered

- Keep all arithmetic in `slices/loan.rs`: rejected because it mixes numeric calculation with validation and accounting output assembly.
- Make `domain-calculations` a generic math or plugin framework: rejected because no such shared abstraction is required.

## Consequences

- `domain-calculations` depends on Money and numeric types, not on Slice configuration types; Loan Slice extracts the configured rate and term and passes those values to the calculations.
- Loan arithmetic can be reviewed independently from account mapping and ledger entry construction.
- Slice remains the owner of active-loan and account-related invariants.
