# ADR-01: Monthly simulation and external run control

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [roadmap](../implementation-roadmap.md), [ADR-16](ADR-16-configuration-and-run-control.md)
- **Test vectors:** N/A; control semantics are exercised in M3 lifecycle tests.

## Context
The simulation needs a uniform time boundary without pausing for domain decisions or treating economic outcomes as failure.

## Decision
The domain advances by one inclusive `YearMonth` transition at a time. The transition consumes a complete opening state, frozen current-period input, and explicit current-period random input, then returns the period result and complete closing state. It has no total-duration or continue/stop policy. A higher-level run controller may repeatedly invoke it and apply an end month, configured month-end condition, or external between-period continue/stop input. Only explicit run controls stop a run.

## Alternatives considered
- Fixed duration or stopping on loan payoff/cash thresholds: conflicts with externally bounded simulation and continuation from arbitrary states.

## Consequences
Run controls belong to the higher-level caller, outside the single-period transition. Negative cash flow and loan payoff alone do not stop a run. Fast and stepwise execution use the same transition.
