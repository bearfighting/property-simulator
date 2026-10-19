# ADR-10: Three delivery phases

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [roadmap](../implementation-roadmap.md), [overview](../overview.md)
- **Test vectors:** N/A; milestone acceptance is maintained in the roadmap.

## Context
The project needs a staged path that protects domain correctness from early UI and persistence coupling.

## Decision
Deliver the core in M0–M5, then application/persistence including CLI and storage, then the GPUI desktop application. Follow roadmap ordering; v1 does not pause for interactive domain decisions.

## Alternatives considered
- Starting with desktop workflows: creates presentation dependencies before core behavior is stable.

## Consequences
CLI, persistence, and desktop are later phases and must call the core through application boundaries.
