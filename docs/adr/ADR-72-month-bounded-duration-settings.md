# ADR-72: Month-bounded duration settings

- **Status:** Accepted
- **Date:** 2026-10-26
- **Related:** [ADR-35](ADR-35-ordinal-month-periods.md), [ADR-58](ADR-58-stop-condition-domain-boundary.md), [Domain model](../domain-model.md)
- **Verification examples:** stop thresholds, loan terms, repair delays, and persisted month ordinals accept values through 65,535 and reject larger values; checked month arithmetic still reports overflow when a particular operation exceeds that range.

## Context

`Month` stores a zero-based ordinal in `u16`, while some duration inputs were
accepted as `u32`. This allowed a configured stop threshold that could never be
reached and loan or repair settings that could not be represented by the
simulation period model.

## Decision

- Keep `Month(u16)` and use its full representable range; do not introduce a
  smaller product duration limit.
- Bound automatic month-count stop conditions, loan terms, repair delays, and
  persisted period ordinals to 65,535. Validate wider public inputs when their
  typed Core parameters are constructed, and reject out-of-range stored values
  at the SQLite boundary.
- Continue to use checked arithmetic for calculated periods and repair due
  months. A valid duration can still overflow when added to a late occurrence
  month; that calculation fails explicitly rather than wrapping.

## Alternatives considered

- Widen `Month` and all related persistence and calculation contracts: deferred
  because the existing bound already exceeds the intended use by a wide margin.
- Accept unbounded `u32` durations and fail only when a calculation reaches an
  unrepresentable month: rejected because some thresholds would be impossible
  to satisfy and invalid settings could be saved successfully.

## Consequences

- Stop-condition DTOs and UI parsing use the same positive `u16` range as the
  domain Month ordinal.
- Loan and repair constructors report an explicit range error for values above
  the supported period range.
- SQLite checks align the persisted month and duration bounds with the Core
  contracts. Startup adds this specific configuration marker to an older
  database when needed and preserves existing non-empty stop-condition sets.
  This targeted compatibility step does not establish general schema
  versioning or migration guarantees.
