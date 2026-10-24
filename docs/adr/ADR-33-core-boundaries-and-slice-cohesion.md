# ADR-33: Core value boundaries and Slice cohesion

- **Status:** Accepted direction; detailed APIs remain open
- **Date:** 2026-10-22
- **Scope:** Core module responsibilities and dependency boundaries

## Context

The existing Core separates primitive values, accounting, models and simulation into crates, but the responsibilities do not align cleanly with the concepts they implement. Slice behavior is spread across Slice modules, monthly dispatch, validation and settlement. The previous Slice-runtime decision made centralized dispatch a fixed design before these responsibilities and interfaces were clear.

## Decision

- Treat `Money`/`Currency`, `YearMonth`, and typed identifiers as three focused foundational crate responsibilities: `money`, `time`, and `id`.
- Do not preserve a general-purpose `models` crate by default. A calculation, distribution or sampler belongs with the Slice or domain behavior that owns its meaning, unless a concrete shared responsibility justifies a separate module.
- Keep domain behavior cohesive. Accounting and Ledger rules, Slice behavior, random input, Context and Snapshot each need explicit ownership and interfaces before code migration.
- Each Slice owns its pure domain calculation and returns its own typed result and journal entries. Application selects, creates, invokes, and gathers the required Slice results, then appends their entries to a candidate Ledger and records the monthly result. Core does not define a cross-Slice business aggregator; no Slice may depend on another Slice's execution order. Cross-month use-case flow and lifecycle coordination belong to Application.
- Use the standalone `core/` Cargo workspace as the clean Core design area. Reuse old code only after its responsibility and dependencies fit the agreed interfaces.

## Alternatives

- Preserve the existing `domain`/`models` split: rejected as the target because its crate names are too broad to explain ownership and the current contents mix state with calculations.
- Keep improving the existing monthly dispatcher before defining Slice interfaces: rejected because it risks preserving the disputed responsibility split.
- Move the whole old implementation at once: rejected because it would carry over unclear coupling and test-only orchestration.

## Consequences

- The existing crate layout and current monthly entry point are implementation history, not the target architecture.
- New interfaces and dependency arrows must be agreed before porting Slice execution and context/snapshot behavior.
- Existing domain rules and accounting semantics remain requirements unless separately changed; this decision changes their organization, not their meaning.
