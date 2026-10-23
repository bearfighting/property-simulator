# ADR-31: Stateless Slice runtime and deferred repair costs

- **Status:** Superseded for architecture by [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md); retained as history
- **Date:** 2026-10-21
- **Scope:** Slice invocation, monthly Ledger flow, and repair occurrence/cost behavior

## Context

The current monthly API requires callers to assemble concrete Slice adapters and routes loans separately. Activity configurations and samples use central enums, while some Slice behavior can generate a same-month follow-up activity. Repair probability and repair cost also represent different random questions.

## Historical decision

Slice implementations and runtime instances are stateless. A simulation's Slice instance collection is data, not an implementation registry. `ActivityConfiguration` is the sole closed Slice-type tag; use a closed sample-value enum only where heterogeneous samples cross the monthly boundary. Keep per-Slice fields and behavior in each Slice module, and map Core values to DTOs in Application rather than serializing Core types. The single-month boundary dispatches the collection through one protocol, and every Slice, including loans, follows the same invocation flow. Slice outputs are typed proposals; no Slice owns or mutates the shared Ledger. Loan continuation state and calculated loan outputs remain separate from random samples. `MonthlyResult` exposes typed lifecycle changes with an explicit effective month; user enablement remains context data.

Apply accepted user settings before monthly calculation. For changes to booked facts, derive a read-only effective opening Ledger view from the opening Ledger plus explicit adjustment proposals. Settle those proposals together with ordinary Slice postings after calculation. Thus all Slices see the same effective opening view, and month-end Ledger projections drive cash flow, snapshot and stop evaluation.

Use one temporal convention for all Slices: freeze inputs at month start; give each event, effect, generated instance and posting an explicit effective `YearMonth`; settle that month's Ledger and form its full snapshot at month end. Any delayed output is retained as a pending item until its effective month. Use one Application-provided ID allocator for user and generated instances and outputs, producing opaque UUID v4 values from randomness independent of Slice sampling; provenance is stored separately. Map the transient Core result to one full snapshot DTO and serialize it in Data Access; do not persist duplicate result records or put SQLite serialization in Core. Ordinary cash expenses debit an expense account and credit cash; do not add period-end capitalization or closing entries unless a domain behavior requires them.

Model repair with two distinct Slice types and a cross-month precedence relation: RepairOccurrence samples whether an event occurs in M and may generate a one-time RepairCost instance. The cost instance reads the occurrence month from its snapshot/provenance and uses its own delay parameter N to become effective in M+N (initially N may be 1). RepairCost does not sample whether a repair occurs; it samples the amount conditional on its source event, posts the expense against cash and completes in its effective month. RepairOccurrence continues its monthly judgments while costs are pending, so multiple pending cost instances are valid. The snapshot retains pending instances; if the simulation stops before settlement, no future Ledger posting is fabricated. The interval does not reset at occurrence; once a RepairCost settles, its effective month becomes the latest repair-cost month and resets the interval baseline. The baseline is the later of simulation start and the latest known settled RepairCost month; no prior event or transaction is invented. The monthly rate increases with completed months and remains below 100%; the model parameters are specified by the runtime contract and test vectors.

## Alternatives

- A static factory directory: rejected because it duplicates the simulation's instance collection and centralizes instance creation.
- Caller-created per-Slice adapters and a special loan loop: rejected because adding an activity would require changes across orchestration boundaries.
- One repair distribution for both occurrence and severity: rejected because time-dependent occurrence and repair cost are separate modeled quantities.
- A Ledger layer that samples or generates economic events: rejected because event decisions belong to the originating Slice; Ledger settlement only validates and applies postings.

## Consequences

The cross-month repair behavior and its provenance remain domain behavior to preserve. The centralized dispatch, shared configuration/sample enums, and related module boundaries are not current architecture requirements. See [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md) for the current direction.
