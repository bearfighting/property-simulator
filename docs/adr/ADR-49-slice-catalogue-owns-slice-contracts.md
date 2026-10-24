# ADR-49: Slice catalogue owns Slice contracts

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [ADR-41](ADR-41-application-available-slice-kinds.md), [ADR-46](ADR-46-explicit-slice-construction-and-application-composition.md), [ADR-48](ADR-48-monthly-snapshot-content-and-continuation.md), [Project structure](../project-structure.md)

## Context

Application must construct each period's Slice instances, and Snapshot must retain their typed outputs and provenance. Keeping per-period inputs in the Slice implementation crate would make the shared Application/Slice contract less visible. Keeping events and effects in Snapshot would also make Slice and Snapshot depend on different owners for the same output contract.

## Decision

- `slice-catalogue` defines the closed list of supported `SliceKind` values, user settings and parameters, per-period typed input values, and shared output contracts (`Event`, `Effect`, and `SliceResult`).
- `SliceResult` carries its `SliceKind`, events, effects, and journal entries. Construction validates that each event and effect belongs to the declared kind.
- Snapshot stores `SliceResult` values grouped by kind, preserving which Slice produced each output and journal increment.
- `slice-catalogue` may depend on shared foundation and accounting value types, but must not depend on Context, Snapshot, Application, or Slice implementations. Context, Snapshot, and Slice implementations may depend on the catalogue.
- Concrete Slice calculations remain in the Slice crate. Application assembles each period's input from Context, Ledger, Snapshot, and account mappings.

## Alternatives considered

- Keep the catalogue limited to user settings: rejected because per-period inputs and outputs are also contracts shared across Application, Slice, and Snapshot.
- Put shared event/output types in Snapshot: rejected because Slice would depend on the monthly persistence/result container to produce its outputs.
- Put each input type only in its Slice implementation module: rejected because Application constructs those values and needs a stable typed interface.

## Consequences

- The catalogue depends on `AccountCode`, `Month`, `Money`, and `JournalEntry` types used by the Slice contracts.
- Snapshot no longer owns separate Event and Effect definitions; it stores the catalogue's per-Slice results.
- Adding a Slice requires adding its kind-specific configuration/input/output contract before its behavior implementation.
