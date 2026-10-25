# ADR-60: Core simulation contracts without aggregate state types

- **Status:** Accepted
- **Date:** 2026-10-24
- **Supersedes:** Core ownership of Context/Snapshot in [ADR-39](ADR-39-context-settings-and-month-zero-start.md), [ADR-48](ADR-48-monthly-snapshot-content-and-continuation.md), [ADR-49](ADR-49-slice-catalogue-owns-slice-contracts.md), and [ADR-50](ADR-50-context-is-source-of-current-settings.md)
- **Related:** [ADR-41](ADR-41-application-available-slice-kinds.md), [ADR-58](ADR-58-stop-condition-domain-boundary.md), [Domain model](../domain-model.md)
- **Verification:** Core has no Context or Snapshot aggregate crate; `SliceResult` rejects Month 0 and JournalEntries from another Month; configuration collections are owned by Application.

## Context

Core `Context` and `Snapshot` grouped settings and multiple Slice results for
cross-month application flow. They did not represent a single domain behavior.
The heterogeneous `SliceSetting` collection was only consumed by Context,
while individual Slice parameter and input types are required by Core
calculations.

## Decision

- Remove Core Context, UserSettings, InitialValues, SimulationName, and
  Snapshot aggregate types. Application owns simulation configuration and
  organizes monthly facts; Data Access persists those facts by their own
  responsibilities.
- Keep each Slice's typed parameters, period inputs, `SliceKind`, `Event`, and
  the single-invocation `SliceResult` contract in `slice-catalogue`. Effect
  continuation records were subsequently removed by ADR-64. Do not
  keep a heterogeneous `SliceSetting` or
  `SliceSettings` collection in Core; Application may define one if a use case
  needs it.
- Include Month in `SliceResult` and validate that every included
  `JournalEntry` belongs to that Month. Events must match the declared
  `SliceKind`.
- Keep the catalogue as a closed type contract, not a dispatcher or a
  cross-Slice result aggregator. Application invokes each Slice and collects
  its result.

## Alternatives considered

- Keep Context and Snapshot as in-memory-only Core values: rejected because
  they still aggregate application state and cross-Slice results in Core.
- Keep `SliceSettings` in the catalogue for convenient loading: rejected
  because it is a heterogeneous configuration collection used by application
  coordination, not by individual Slice calculations.
- Validate JournalEntry months only in Application: rejected because the
  single-Slice result contract can enforce its own period invariant directly.

## Consequences

- Application constructs Slice parameters and inputs from independent stored
  facts and gathers per-Slice results by Month.
- The per-Slice result contract stays useful without a Core Snapshot type.
- Removing the aggregate types does not change Ledger or reporting
  responsibilities.
