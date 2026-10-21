# ADR-10: Delivery phases

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [roadmap](../implementation-roadmap.md), [overview](../overview.md)
- **Test vectors:** N/A; milestone acceptance is maintained in the roadmap.

## Context
The project needs a staged path that protects domain correctness from early UI and persistence coupling.

## Decision
Delivery sequencing is clarified by ADR-29 and the implementation roadmap: M0–M5 establish the foundations and complete independent Core modules; M6 integrates one monthly calculation; later stages add Application/Data Access, then CLI/GPUI presentation. Interactive domain decisions and pauses are outside the current product scope.

## Alternatives considered
- Starting with desktop workflows: creates presentation dependencies before core behavior is stable.

## Consequences
CLI, persistence, and desktop are later stages. Presentation calls Application; Application coordinates Data Access and monthly calculation.
