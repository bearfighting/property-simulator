# ADR-03: Start from a valid opening state

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md)
- **Test vectors:** N/A; opening-state completeness belongs to M1/M3 validation tests.

## Context
Users may begin with an already owned or financed property and incomplete pre-simulation history.

## Decision
Accept any sufficiently complete, structurally valid opening state. Preserve known facts and continuation variables; do not invent past events or transactions.

## Alternatives considered
- Requiring a full acquisition and repayment history: excludes valid opening snapshots and fabricates unsupported history.

## Consequences
Opening balances and state are distinct from generated period activity and are validated for enabled slices.
