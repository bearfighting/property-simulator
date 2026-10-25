# ADR-57: Per-Slice settings persistence

- **Status:** Accepted direction
- **Date:** 2026-10-24
- **Related:** [ADR-41](ADR-41-application-available-slice-kinds.md), [ADR-42](ADR-42-user-settings-and-opening-values.md), [SQLite schema](../../data-access/sqlite/README.md)
- **Verification:** each supported Slice configuration maps to its dedicated typed table; Loan terms remain unchanged across a latest-settings update; a later-month tombstone removes an earlier Slice configuration.

## Context

Core `SliceKind` and `SliceSetting` form a closed, statically defined set. The
configuration fields differ by Slice, and a generic persisted `UserSettings`
blob would hide those types and repeat the Core aggregate boundary in storage.
Opening monetary facts are recorded in Month 0 JournalEntry rows, while loan
terms are configuration belonging to the Loan behavior.

## Decision

- Do not persist a generic `settings` aggregate table or opaque whole-settings
  payload.
- Persist each Slice's user configuration in a dedicated typed table, keyed by
  simulation and effective starting month. The first effective month is Month 1,
  because Month 0 is initialization and has no Slice calculation. Values in a
  row replace that Slice's configuration from that month onward; an all-null row is a removal
  tombstone. The greatest applicable effective month determines the value for
  a calculation month.
- Persist stop conditions separately as a simulation-level condition set at an
  effective month beginning at Month 1; do not attach them to an individual
  Slice setting.
- Keep property value and opening loan balance in Month 0 Ledger facts. Keep
  fixed loan terms with the Loan setting and reject changes after initialization.
  The configured remaining term is measured at simulation start; monthly
  remaining term is separate continuation input.
- Treat concrete per-month `SliceInput` values as runtime values assembled from
  Ledger, Month, Effects, configuration, account mappings, and injected random
  input; do not persist a duplicate copy of derivable inputs.

## Alternatives considered

- Persist the whole `UserSettings` value as one blob: rejected because it hides
  the closed per-Slice types and duplicates the Core aggregate in storage.
- Use one generic table with `slice_kind`, nullable `enabled`, and an opaque
  settings blob: rejected because the set of Slice kinds is static and each
  kind has different fields and enablement rules.

## Consequences

- Adding a configured Slice requires a dedicated schema table and DTO mapping,
  consistent with the finite compile-time Slice catalogue.
- Settings updates are represented as effective-month changes per Slice;
  multiple edits targeting the same not-yet-calculated month replace that
  month's row rather than retaining command history.
- Core `InitialValues` contains only opening currency/property/debt facts;
  Loan parameters are part of `SliceSetting::Loan` and remain immutable in
  `Context::with_latest_settings`.
