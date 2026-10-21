# ADR-28: Pre-M5 crate call-boundary review

> **Current scope clarification (ADR-29):** This review removed premature production run/coordinator composition. M5 completes independent Core modules; M6 adds stateless single-month integration. Application orchestration and Data Access remain later layers. The historical review findings below remain valid.

- **Status:** Accepted
- **Date:** 2026-10-21
- **Related:** [roadmap](../implementation-roadmap.md), [architecture](../architecture.md), [ADR-05](ADR-05-slice-coordination.md), [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md)
- **Test vectors:** The review changes no domain rule; affected module-boundary tests are identified during the review.

## Context

The workspace already has several crates and real calls between them. A Cargo dependency edge alone does not show whether modules have been composed into a complete execution flow. Existing names such as `coordinator` also do not, by themselves, establish that the current composition boundary is the intended design. M5 must not build on an unreviewed assumption about those calls.

## Decision

- Before M5, inspect both the declared crate dependency graph and actual cross-crate calls. Record which calls exchange types or invoke behavior, and what responsibility each call serves.
- Modules may reference shared data types and stable interfaces. A module must not directly call another responsibility module's concrete implementation. When collaboration is needed, the caller constructs and injects a narrow interface; the consumer invokes only that injected interface. Use explicit input/output functions where no collaborator is needed, and do not add interfaces mechanically.
- Review each actual call against the responsibilities and interfaces already designed. Extract or reshape concrete cross-module calls so calculation modules can be called independently; leave composition and IO to a future higher-level application module. Do not replace direct calls with an internal coordinator/composer that still owns the combined flow.
- Do not classify every cross-crate call as wrong solely because it exists, and do not preserve a call solely because its current name or location suggests a coordinator role. The decision must follow the concrete responsibility and behavior.
- This stage does not design or add a controller, work unit, CLI/GUI orchestration, persistence, or a new generic integration framework. Those higher-level choices belong to later application design.
- Update the roadmap, dependency/call documentation, and affected tests before M5 begins. M5 uses only boundaries confirmed by this review.

## Alternatives considered

- Start M5 by reusing all existing coordinators: rejected because their actual call boundaries have not yet been reviewed against the current design.
- Remove all cross-crate calls: rejected because some calls may be necessary and appropriately owned calculation collaboration; dependency presence alone is not proof of a boundary violation.
- Introduce an application controller or work unit now: deferred because CLI/GUI application responsibilities have not been designed.

## Consequences

M5 has an explicit prerequisite review and any justified remediation. The current Rust call graph is treated as implementation to inspect, not as a fully approved application/runtime architecture. No new higher-level orchestration layer is implied.

## Initial source review

The current Cargo dependency graph is acyclic:

```text
accounting -> domain
models -> domain
randomness -> domain, models
simulation -> domain, accounting, models
simulation (dev) -> randomness
```

These edges also contain actual behavior calls; data/interface references are allowed, while concrete implementation calls across responsibility modules are not:

| Edge | Actual use found | Initial assessment |
|---|---|---|
| `accounting -> domain` | Ledger and opening-balance code uses domain IDs, money, currency, and time values. | Value-level dependency matching the accounting boundary. |
| `models -> domain` | Loan and distribution calculations use domain money and errors. | Calculation dependency matching model inputs/outputs. |
| `randomness -> models` | `RandomStream` implements the `models::RandomSource` trait. | Interface implementation dependency; allowed. |
| `randomness -> domain` | Stream identity uses trial, slice, activity, and period types. | Typed identity dependency. |
| `simulation -> models` | Sampling uses caller-injected `Sampler` and `RandomSource`; loan terms use injected `LoanCalculation`; repair support and sample checks use injected `RepairCostValidation`. | Calculation collaboration uses caller-provided narrow interfaces. Repair rules remain owned by the model adapter; production simulation does not call concrete distribution validation methods. |
| `simulation -> randomness` | No production dependency. Test fixtures allocate semantic streams using the randomness crate. | The randomness crate is a development dependency; production sampling receives `RandomSource`. |
| `simulation -> accounting` | Production APIs use accounting values such as `LedgerEntry` and accept `LedgerRead` for validation. Concrete Ledger reads, transaction validation, posting, and projection occur only in test support. | Shared values and the injected read interface are allowed. No production simulation module calls Ledger implementation methods. |
| `simulation -> domain` | Simulation inputs, outputs, IDs, money, periods, and resource keys use domain types and their value operations. | Typed calculation data dependency. |

Initial source findings before remediation (the paths below now live under `test_support`):

- `propose_slice` invokes the caller-supplied generic `SimulationSlice` interface after provenance validation. This injected interface call is part of the independent Slice-output boundary.
- `coordinate_month`, `coordinate_month_with_loans`, run control, and archive implementations have moved under `crates/simulation/src/test_support/`. They are compiled only for tests. `MonthlyResult`, run-control input updates, and saved snapshots that depended on this assembled lifecycle are likewise test-only.
- Production input validation reads accounting facts only through `LedgerRead`. Loan-term transformation calls only the injected `LoanCalculation` interface. `RunKeyFactory` and concrete random stream allocation are not production dependencies of simulation.
- Slice resource keys are derived through the shared `ResourceKey::for_account` value operation; property-tax due-date calculation belongs to `PropertyTaxConfig`.

The most consequential current-code findings are:

- [`SimulationRun`](../../crates/simulation/src/test_support/run_control/mod.rs) retained a Ledger, current month/state, update history, latest snapshot, status, and random streams, and called an injected `SingleMonthExecutor`. No production implementation of that executor existed; implementations were in tests. This was a run-control/runtime composition, not a stateless monthly calculation module.
- [`RunKeyFactory`](../../crates/randomness/src/lib.rs) previously retained an `AtomicU64` sequence and allocated streams by call order. The sequence has now been removed; stream identity is derived from explicit trial identity, so this factory no longer retains mutable allocation state.
- [`SimulationArchive`](../../crates/simulation/src/test_support/archive.rs) owned in-memory runs, branches, snapshots, and latest settings. This was outside the current Core boundary in ADR-27 and duplicated the history responsibility planned for later persistence.
- [`coordinate_month`](../../crates/simulation/src/test_support/coordinator.rs) validated and merged Slice outputs, called Ledger validation and posting, derived closing state, and constructed a complete month result. Loan batch composition was a second path. Both are test-only after remediation; ADR-05 is superseded.
- The former crate surface exposed run control, archive, and coordinators from `simulation::lib`. These exports are now gated to test builds; the production API exposes independent calculation modules and value/interface types.

The multi-month composition fixture is now `crates/simulation/src/monthly_composition_tests.rs`; it is included only under `cfg(test)`. Milestone-coded source names and comments were removed. The `"m3"` still used in Rust tests is a cubic-metre unit.

The production runtime/history composition has been removed from the simulation API while its prior behavioral coverage remains in test support. `LedgerRead`, `LoanCalculation`, and `RepairCostValidation` implementations are supplied by the caller that connects those calculations. Repair input support/range checks and proposal sample checks both use that injected adapter; the adapter delegates to model-owned rules. The production proposal ID factory now receives only period, activity instance, and stable output key; neither `trial_id` nor `branch_id` is required to identify or combine outputs. Run, snapshot, archive, and branch identity remain in test support only. No application controller, work unit, or persistence layer is introduced here.

The presence of a state-bearing data value such as `Ledger` is not itself a violation: it may be passed as a calculation input and returned as a new output value. The review concerns objects that retain run/session state or hide mutable sequencing, and concrete behavior calls across responsibility boundaries.
