# Architecture decision records

This directory records decisions that affect domain behavior, architecture, or data contracts.

## Numbering and lifecycle

- Use the next two-digit number (`ADR-NN-title.md`); never reuse a number.
- Keep one decision topic per record. Update its status and consequences if superseded, and link the replacing ADR.
- `Accepted` means the baseline is approved for implementation. `Proposed` requires a later decision. `Superseded` identifies its replacement.
- Keep the roadmap and topic documents authoritative for current scope; update them alongside any consequential change.
- Link shared numerical examples by ID in `docs/test-vectors.md` instead of copying vectors into ADRs.

## Template

Copy [`TEMPLATE.md`](TEMPLATE.md) for new records.

## Index

| ADR | Decision | Status |
|---|---|---|
| [01](ADR-01-monthly-run-control.md) | Monthly simulation and external run control | Accepted |
| [02](ADR-02-monthly-tick.md) | One monthly transactional tick | Accepted |
| [03](ADR-03-arbitrary-opening-state.md) | Start from a valid opening state | Accepted |
| [04](ADR-04-scenario-and-overrides.md) | Scenario preset and user overrides | Accepted |
| [05](ADR-05-slice-coordination.md) | Independent proposals and coordinated commit | Accepted |
| [06](ADR-06-events-effects-ledger.md) | Separate events, effects, and ledger | Accepted |
| [07](ADR-07-monthly-cashflow.md) | Monthly cash-flow posting | Accepted |
| [08](ADR-08-dataset-output.md) | Dataset as core output | Accepted |
| [09](ADR-09-random-trials.md) | Explicit random trials and resampling | Accepted |
| [10](ADR-10-delivery-phases.md) | Three delivery phases | Accepted |
| [11](ADR-11-static-registration.md) | Static Rust registration initially | Accepted |
| [12](ADR-12-independent-random-categories.md) | Independent random categories initially | Accepted |
| [13](ADR-13-ledger-profile.md) | Household property ledger profile | Accepted |
| [14](ADR-14-mortgage-interest.md) | Mortgage interest and payment convention | Accepted |
| [15](ADR-15-random-streams-and-branches.md) | Random streams, branches, and recovery | Accepted |
| [16](ADR-16-configuration-and-run-control.md) | Configuration and run controls | Accepted |
| [17](ADR-17-money-and-identifiers.md) | Currency-tagged money and opaque identifiers | Accepted |
| [18](ADR-18-sufficient-state-transition.md) | Sufficient state and single-period transition | Accepted |
| [19](ADR-19-validation-only-domain-errors.md) | Validation boundary and fatal runtime failures | Accepted |
| [20](ADR-20-composable-slices-and-random-sampling.md) | Composable proposals, stable provenance, and decoupled sampling | Accepted |
| [21](ADR-21-m3-run-randomness-and-persistence.md) | M3 run randomness, persistence, and composition | Accepted |
| [22](ADR-22-m3-run-inputs-snapshot-and-cashflow-contract.md) | M3 run inputs, snapshot references, and cash-flow aggregation | Accepted |
| [23](ADR-23-loan-ledger-account-granularity.md) | Loan Ledger account granularity | Accepted |
| [24](ADR-24-m4-operating-activity-contracts.md) | M4 operating activity contracts | Accepted |
