# ADR-01: Monthly simulation and external run control

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [roadmap](../implementation-roadmap.md), [ADR-16](ADR-16-configuration-and-run-control.md)
- **Test vectors:** N/A; control semantics are exercised in M3 lifecycle tests.

## Context
The simulation needs a uniform time boundary without pausing for domain decisions or treating economic outcomes as failure.

## Decision
Advance in inclusive `YearMonth` intervals. An end month, configured month-end condition, or external month-by-month continue/stop input may bound a run. Only explicit controls stop it.

## Alternatives considered
- Fixed duration or stopping on loan payoff/cash thresholds: conflicts with externally bounded simulation and continuation from arbitrary states.

## Consequences
Run controls are application inputs to the core; negative cash flow and loan payoff alone do not stop a run.
