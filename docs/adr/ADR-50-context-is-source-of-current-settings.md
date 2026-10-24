# ADR-50: Context is the source of current settings

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [ADR-39](ADR-39-context-settings-and-month-zero-start.md), [ADR-48](ADR-48-monthly-snapshot-content-and-continuation.md), [Domain model](../domain-model.md)

## Context

`Context` contains immutable initial settings and an optional latest settings value. `Context::effective_settings()` selects the latest value when present, otherwise the initial value. Storing a second complete `UserSettings` value in every `Snapshot` duplicates Context state, while the current design does not require historical versions of settings to be reconstructed from snapshots.

## Decision

- Application reads the effective settings from Context before constructing the month's Slice instances.
- `Snapshot` stores completed Slice results, including typed state changes, but does not copy `UserSettings` or project Slice-specific continuation fields.
- Context remains the source of current settings. If future behavior requires reconstructing settings as they stood for each historical month, define a narrower month-input record and store that explicitly.

## Consequences

- Snapshot construction no longer accepts user settings.
- Slice construction and snapshot recording must be composed by Application from the same effective Context settings.
- Historical per-month settings are not available from Snapshot under this design.
