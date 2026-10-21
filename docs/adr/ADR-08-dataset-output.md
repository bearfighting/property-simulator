# ADR-08: Dataset as core output

- **Status:** Superseded by [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md) and [ADR-29](ADR-29-layered-calculation-and-application-boundaries.md)
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md)
- **Test vectors:** N/A for the current production boundary; monthly-result composition coverage remains in historical simulation test support. Storage and export tests belong to the later Application/Data Access stage.

## Context
Core results need to remain analyzable and usable by multiple future interfaces.

## Decision
The original proposal made a complete in-memory dataset, snapshots, and branch relationships a Core output. That ownership decision is superseded by ADR-27 and ADR-29: M6 defines one monthly calculation's input/output contract, while history and persistence belong outside the calculation layers. The export examples below remain future application possibilities, not M5/M6 deliverables. Do not generate investment recommendations or prebuilt investment reports.

## Alternatives considered
- UI-specific result objects or report-first output: couples the core to presentation and interpretation.

## Consequences
When an application-level dataset is designed, keep user configuration and generated business records explicit and record the application version once for that dataset. Do not attach independent versions to models or random streams. SQLite and export representations are deferred; this ADR does not prescribe snapshot ownership or persistence layout.
