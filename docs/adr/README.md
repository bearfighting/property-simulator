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
| [01](ADR-01-monthly-run-control.md) | Monthly simulation and external run control | Superseded by ADR-27 |
| [02](ADR-02-monthly-tick.md) | One monthly transactional tick | Accepted |
| [03](ADR-03-arbitrary-opening-state.md) | Start from a valid opening state | Accepted |
| [04](ADR-04-scenario-and-overrides.md) | Configuration presets and user overrides | Configuration source model retained; scenario/branch lifecycle superseded by ADR-26/27/29 |
| [05](ADR-05-slice-coordination.md) | Independent proposals and coordinated commit | Superseded by ADR-28 |
| [06](ADR-06-events-effects-ledger.md) | Separate events, effects, and ledger | Accepted |
| [07](ADR-07-monthly-cashflow.md) | Monthly cash-flow posting | Accepted |
| [08](ADR-08-dataset-output.md) | Dataset as core output | Superseded by ADR-27/29 |
| [09](ADR-09-random-trials.md) | Explicit sampling inputs and diagnostics | Sampling contract retained; trial/branch workflow superseded by ADR-27/29 |
| [10](ADR-10-delivery-phases.md) | Staged delivery sequence | Accepted |
| [11](ADR-11-static-registration.md) | Static Rust registration initially | Accepted |
| [12](ADR-12-independent-random-categories.md) | Independent random categories initially | Accepted |
| [13](ADR-13-ledger-profile.md) | Household property ledger profile | Accepted |
| [14](ADR-14-mortgage-interest.md) | Mortgage interest and payment convention | Accepted |
| [15](ADR-15-random-streams-and-branches.md) | Random streams and branches | Sampling details retained; branch workflow superseded by ADR-27/29 |
| [16](ADR-16-configuration-and-run-control.md) | Configuration and run controls | Superseded by ADR-27/28 |
| [17](ADR-17-money-and-identifiers.md) | Currency-tagged money and opaque identifiers | Accepted |
| [18](ADR-18-sufficient-state-transition.md) | Sufficient state and single-period transition | Accepted |
| [19](ADR-19-validation-only-domain-errors.md) | Validation boundary and fatal runtime failures | Accepted |
| [20](ADR-20-composable-slices-and-random-sampling.md) | Composable proposals, stable provenance, and decoupled sampling | Accepted |
| [21](ADR-21-m3-run-randomness-and-persistence.md) | Caller-provided random inputs and sampling boundaries | Sampling contract retained; M3 run/persistence workflow historical under ADR-27/29 |
| [22](ADR-22-m3-run-inputs-snapshot-and-cashflow-contract.md) | Monthly inputs and Ledger-derived cash flow | Current calculation boundary by ADR-27; M5/M6 allocation by ADR-29 |
| [23](ADR-23-loan-ledger-account-granularity.md) | Loan Ledger account granularity | Accepted |
| [24](ADR-24-m4-operating-activity-contracts.md) | M4 operating activity domain contracts | Accepted; runtime composition historical |
| [25](ADR-25-m5-in-memory-core-contract.md) | Earlier M5 in-memory core data contract | Archive scope superseded by ADR-27; M5/M6 allocation clarified by ADR-29 |
| [26](ADR-26-scenario-scoped-runtime-and-concurrency.md) | Single-threaded calculation scope; scenario management deferred | Accepted |
| [27](ADR-27-current-month-runtime-and-database-owned-history.md) | Monthly calculation boundary and Ledger-derived cash flow | Accepted |
| [28](ADR-28-pre-m5-crate-call-boundary-review.md) | Pre-M5 crate call-boundary review | Accepted |
| [29](ADR-29-layered-calculation-and-application-boundaries.md) | Core, monthly integration, application and data-access boundaries | Accepted |
| [30](ADR-30-monthly-integration-inputs-and-atomic-settlement.md) | M6 caller-injected Slice adapters and atomic Ledger settlement | Accepted |
