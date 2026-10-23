# ADR-41: Application-available Slice kinds

- **Status:** Accepted direction; parameter APIs remain open
- **Date:** 2026-10-23
- **Related:** [Domain model](../domain-model.md), [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md)
- **Verification:** `SliceKind` exhaustively names supported Slice behaviors; `SliceSetting` binds each kind to applicable per-Slice parameters and user enablement, while immutable loan terms are part of `InitialValues`.

## Context

The Slice implementation may contain internal calculations and generated instances in addition to behaviors with their own configuration. Context settings need a stable type for supported Slice parameters without exposing every internal calculation as a configurable kind. A generated instance can still require its own Slice settings, independently of whether the user directly enables that kind.

## Decision

- Define `SliceKind` in the `slice-catalogue` crate as a closed enum of Slice kinds whose parameters are supplied through Context and may be used by Application's simulation flow.
- The initial kinds are `Loan`, `PropertyTax`, `Rental`, `RepairOccurrence`, `RepairCost`, and `Utility`.
- A kind's presence in `SliceKind` means the simulation supports its behavior and settings. It does not imply that the user directly enables it; settings and invocation lifecycle determine use.
- `RepairOccurrence` has user-controlled enablement. Disabling it prevents new repair occurrences from generating cost instances.
- Include `RepairCost`: although instances may be generated from a repair occurrence, the kind has its own Context-supplied settings and no independent user enable flag. Already generated instances follow their own lifecycle.
- The catalogue does not create or evaluate Slice instances. Parameter structs define the type boundary; concrete fields for most kinds and their validation rules remain to be designed.
- Do not introduce a `SliceGroup` or general dependency/order mechanism yet. Record this as technical debt and revisit it when concrete cross-Slice dependency requirements are known.

## Alternatives considered

- Include every Slice implementation module in the enum: rejected because internal or generated behavior would become a high-level configuration choice.
- Exclude generated Slice kinds: rejected because generation of an instance does not eliminate its independent settings contract.
- Use string names or runtime registration: rejected because the supported set is currently closed and compile-time exhaustive matching is useful.

## Consequences

- Adding a supported Slice with Context-supplied settings requires adding a `SliceKind` variant and updating exhaustive consumers.
- Internal Slice modules can change without expanding the Application-facing catalogue.
- Which kinds the user can directly enable, and which are invoked only as generated activity instances, remains a separate setting/lifecycle decision.
- Concrete parameter fields and validation rules for most kinds remain open design work; the closed `SliceSetting` enum binds each kind to its per-Slice setting and `SliceSettings` derives map keys from the enum variant. Loan terms are immutable opening settings.
- If actual Slice dependencies require ordering or grouping, a later decision must define that behavior without assuming arbitrary order for independent Slices.
