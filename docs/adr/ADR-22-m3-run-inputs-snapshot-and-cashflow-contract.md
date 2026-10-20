# ADR-22: M3 run inputs, snapshot references, and cash-flow aggregation

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related:** [roadmap](../implementation-roadmap.md), [domain model](../domain-model.md), [ADR-13](ADR-13-ledger-profile.md), [ADR-14](ADR-14-mortgage-interest.md), [ADR-16](ADR-16-configuration-and-run-control.md), [ADR-20](ADR-20-composable-slices-and-random-sampling.md), [ADR-21](ADR-21-m3-run-randomness-and-persistence.md)
- **Test vectors:** M3 covers command validation and stop behavior, parent snapshot preservation, per-instance cash-flow attribution, and aggregation invariant under permutation and regrouping.

## Context

M3 needs a concrete backend boundary for run commands, completed snapshots and branch history, plus a result contract for contributions from composable Slice instances. It also needs a clear relationship between those contributions and the double-entry Ledger.

## Decision

- Run control is a backend API. Each invocation receives explicit user command parameters, including applicable configuration updates and whether to continue or stop. It validates updates, records their effective period, and decides whether to invoke the next monthly transition. A UI or CLI is not part of M3.
- Completed monthly snapshots and results are addressable records. Branch history is represented by branch identity and parent snapshot references; a dedicated tree container and loading every branch together are not required. A tree view can be derived from these references.
- Monthly results retain each active Slice instance's cash-flow contribution and provenance. Contributions describe the cash impact of committed economic activity and reconcile to the direct cash-flow projection from posted Ledger entries; they are not a competing cash balance or a substitute for postings. Period and cumulative totals are derived sums. Addition has an identity, is associative, and is commutative; reordering or regrouping contributions cannot change totals.
- This cash-flow aggregation guarantee does not require arbitrary state transitions to commute. Existing resource/dependency validation governs shared state effects.
- A fixed-rate, fully amortizing loan is an ordinary Slice instance whose opening balance is an in-scope liability fact. M3 uses ADR-14 for mortgages and other installment loans with those same terms: required calculation inputs are opening balance, remaining term, and fixed annual rate; the model derives the scheduled payment, interest expense, actual cash payment, principal reduction, and closing liability balance. The effective month is the first interest and payment month: it takes effect on day one, accrues a full month's interest, and makes the first payment at month-end. The monthly Slice effects and balanced Ledger transaction(s) update liability and cash; principal is financing cash flow and interest paid is operating cash flow. The continuation balance must reconcile to the Ledger liability. Implementations may use one liability account per loan or a shared liability account with traceable allocations. Revolving, interest-only, variable-rate, and balloon structures are outside the M3 calculation scope and require their own convention before monthly behavior is modeled.
- M3 does not model depreciation or amortization of property or other assets. These are non-cash valuation/expense rules and are outside the initial cash-flow scope; property carrying value remains the explicit opening/reference value unless a separately versioned valuation rule changes it.
- The first stochastic model is an implementation choice and does not require a pre-M3 model-selection decision.

## Alternatives considered

- Require a dedicated in-memory branch tree: rejected because parent snapshot references are sufficient and can produce a tree view without fixing storage to that representation.
- Require a separate Ledger account for every loan instance: not required as a chart layout; a shared liability account is acceptable if the sum of instance balances reconciles and provenance is retained.
- Require state transformations to commute: rejected because the specified algebraic invariant concerns cash-flow aggregation; shared state changes remain explicitly coordinated.

## Consequences

M3 can implement and test a small backend run API and record-based snapshot history without committing to a persistence engine or UI workflow. Results remain attributable to Slice instances while aggregate cash flow is independent of execution order and ties to Ledger cash postings. Loan continuation state and liability balances reconcile; the exact account granularity is an implementation choice. Depreciation and amortization remain outside the initial model.
