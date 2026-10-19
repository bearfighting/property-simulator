# ADR-12: Independent random categories initially

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-15](ADR-15-random-streams-and-branches.md)
- **Test vectors:** M3 tests verify category stream separation; no correlation fixture in v1.

## Context
Random models span distinct property and operating domains, but no correlation model is specified.

## Decision
Organize streams by environment, physical property/component, policy/macro, and occupancy/operations. Treat categories as independent initially. Explicit generated shared state may be read, but hidden cross-category random correlation is prohibited.

## Alternatives considered
- Implicit correlations: hard to audit and unsupported by a calibrated model.

## Consequences
Any later correlation mechanism must be explicit, justified, and versioned.
