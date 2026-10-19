# ADR-04: Scenario preset and user overrides

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-16](ADR-16-configuration-and-run-control.md)
- **Test vectors:** N/A; configuration validation cases are specified with M3 inputs.

## Context
Scenarios need reusable starting configuration while retaining user control and clear provenance.

## Decision
Use a Scenario preset as a starting point with explicit user overrides. Separate opening facts from future assumptions and validate required inputs at the owning slice boundary.

## Alternatives considered
- Implicit defaults or one undifferentiated configuration: obscures source and completeness.

## Consequences
Expanded effective configuration and input provenance must be retained for each branch.
