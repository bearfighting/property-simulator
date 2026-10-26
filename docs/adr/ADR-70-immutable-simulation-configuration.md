# ADR-70: Immutable configuration during a simulation

- **Status:** Accepted
- **Date:** 2026-10-26
- **Related:** [Domain model](../domain-model.md), [UI input defaults](../ui-ux-design.md), [Implementation roadmap](../implementation-roadmap.md), [ADR-56](ADR-56-stop-conditions-and-month-boundaries.md), [ADR-57](ADR-57-per-slice-settings-persistence.md), [ADR-65](ADR-65-persist-pending-repair-cost-instances.md)
- **Scope:** User configuration lifecycle within one simulation

## Context

Allowing settings to change during a simulation requires deciding which fields
may change, defining effective months, preserving past inputs, and handling
generated activities that captured earlier settings. The product can instead
make a simulation one complete run with a fixed configuration.

## Decision

- A simulation's configuration is fixed when initialization is confirmed and
  remains unchanged for the lifetime of that simulation.
- Configuration changes do not apply to an active simulation. To use different
  assumptions, the user starts a separate simulation with its own configuration.
- Monthly advancement does not accept configuration updates. A month uses the
  simulation's fixed settings; each completed month's actual inputs and results
  remain persisted for continuation and audit.
- `UserStop` remains an independent user action at the month boundary; it is not
  a configuration edit.
- The first implementation does not need settings versioning, future effective
  months, or a configuration-change history. Whether and how to create another
  simulation is a separate product workflow.
- This decision supersedes the assumptions in ADR-56 that settings updates may
  accompany monthly advancement, and ADR-57/ADR-65 that settings may be edited
  for future effective months. ADR-65's repair-instance persistence decision
  remains in force.

## Alternatives considered

- Allow selected fields to change between months: rejected because it requires
  field-by-field mutability rules and complicates audit and generated activity
  continuation.
- Version all settings by effective month: rejected because the product does
  not need mid-simulation configuration changes.
- Treat the whole multi-month run as one database transaction: rejected because
  completed months must be saved independently for continuation and audit; the
  fixed-configuration rule concerns user inputs, not storage atomicity across
  the full simulation duration.

## Consequences

- The active simulation UI offers monthly advancement and stopping, not settings
  editing. Initialization remains the place to review and commit its settings.
- Application does not need a context-update use case for changing simulation
  configuration or a settings history keyed by effective month.
- Each month remains a separately calculated and persisted unit. Storage
  failures stop further advancement without undoing previously completed months.
- A future workflow for starting another simulation may reuse the initialization
  form, without changing the configuration of an existing simulation.
