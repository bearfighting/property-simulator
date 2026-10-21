# ADR-12: Independent random categories initially

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-15](ADR-15-random-streams-and-branches.md)
- **Test vectors:** M3 tests verify category stream separation; the current model has no cross-category correlation fixture.

## Context
Random models span distinct property and operating domains, but no correlation model is specified.

## Decision
Organize streams by environment, physical property/component, policy/macro, and occupancy/operations. Treat categories as independent initially. A Slice instance may read shared environmental or policy state only when it is part of the immutable opening state or the frozen period input; it cannot read a state/result produced by another Slice instance during the current month. Hidden cross-category random correlation is prohibited.

## Alternatives considered
- Implicit correlations: hard to audit and unsupported by a calibrated model.

## Consequences
Any later correlation mechanism must be explicit and justified.
