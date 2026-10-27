# ADR-56: Stop conditions and month boundaries

- **Status:** Superseded by [ADR-70](ADR-70-immutable-simulation-configuration.md) and [ADR-71](ADR-71-in-memory-simulation-lifecycle-coordinator.md)
- **Date:** 2026-10-24
- **Related:** [ADR-42](ADR-42-user-settings-and-opening-values.md), [Domain model](../domain-model.md)
- **Historical verification:** `AfterMonths(1)` completes Month 1 and prevents Month 2 from starting.

> This ADR records an earlier design. Manual stop is now an in-memory lifecycle
> operation, not a `StopCondition`; configured stop conditions are immutable
> automatic conditions as described by ADR-70 and ADR-71.

## Context

The initial design kept an explicit user stop command separate from configured
`StopCondition` values. This left two stop paths with different representations
even though both take effect before another month is calculated. Stop
conditions also need a persistence representation separate from the settings
payload.

## Decision

- Model manual stopping as `StopCondition::UserStop` alongside `AfterMonths` and
  `AllLoansPaidOff`.
- Evaluate all stop conditions at the boundary before starting the next month.
  A condition satisfied by Month N prevents Month N+1 from starting.
- Persist each condition as a row associated with a complete settings state.
  `AfterMonths` stores its positive `u32` threshold in `value`; the other
  conditions have a null `value`.
- Do not retain a separate stop-command history.

## Alternatives considered

- Keep manual stop as a separate command path: rejected because it duplicated
  the same month-boundary stop behavior with another persisted or transient
  representation.
- Encode conditions inside the settings payload: rejected because the
  relational schema benefits from typed condition rows and constraints on the
  optional threshold value.

## Consequences

- Core `StopCondition` includes `UserStop`; Application applies it as a complete
  settings update and checks it before the next month starts.
- Data Access stores condition rows per settings state and reconstructs the
  `Vec<StopCondition>` when loading settings.
- This supersedes ADR-42's earlier distinction that a manual stop remains a
  command rather than a condition.
