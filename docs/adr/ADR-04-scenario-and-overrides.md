# ADR-04: Configuration presets and user overrides

- **Status:** Configuration source model retained; scenario and branch lifecycle superseded by ADR-26, ADR-27, and ADR-29
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-26](ADR-26-scenario-scoped-runtime-and-concurrency.md), [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md), [ADR-29](ADR-29-layered-calculation-and-application-boundaries.md)
- **Test vectors:** N/A; configuration validation cases are specified with M3 inputs.

## Context
Users need a clear way to provide starting settings and explicit overrides. Earlier design work called these reusable presets “scenarios” and coupled them to branch provenance. Current milestones define calculation inputs and outputs, not runtime scenario or branch management.

## Decision
Configuration may be assembled from a preset and explicit user overrides. Keep opening facts distinct from future assumptions, and validate required inputs at the owning input boundary. This is a configuration source convention only: it does not create runtime scenario identity, branch history, or per-branch provenance storage. Scenario ownership and any branching workflow belong to a future application design, if needed.

## Alternatives considered
- Implicit defaults or one undifferentiated configuration: obscures source and completeness.

## Consequences
The effective configuration and relevant input provenance are explicit calculation inputs. This ADR does not require retaining expanded configuration per branch or prescribe persistence. Runtime scenario and branch lifecycle are superseded by ADR-26, ADR-27, and ADR-29.
