# ADR-01: Monthly simulation and external run control

- **Status:** Superseded by [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md)
- **Date:** 2026-10-19
- **Related:** [roadmap](../implementation-roadmap.md), [ADR-16](ADR-16-configuration-and-run-control.md)
- **Test vectors:** N/A; control semantics are exercised in M3 lifecycle tests.

## Context
The simulation needs a uniform time boundary without pausing for domain decisions or treating economic outcomes as failure.

## Decision
The domain calculation uses one inclusive `YearMonth` as its time unit. The original proposal for a higher-level run controller and its stop conditions is historical; current calculation modules do not define a run or its continuation policy. ADR-27 supersedes that application-flow proposal.

## Alternatives considered
- Fixed duration or stopping on loan payoff/cash thresholds: conflicts with externally bounded simulation and continuation from arbitrary states.

## Consequences
Run controls belong to the higher-level caller, outside the single-period transition. Negative cash flow and loan payoff alone do not stop a run. Fast and stepwise execution use the same transition.
