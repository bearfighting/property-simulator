# ADR-39: Context settings and Month 0 start

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [Domain model](../domain-model.md), [Architecture](../architecture.md), [ADR-34](ADR-34-simulation-random-root-lifecycle.md), [ADR-35](ADR-35-ordinal-month-periods.md)
- **Verification examples:** Initial Context has no latest-settings override; effective settings then equal initial settings; setting a latest value preserves initial settings and root; every new simulation begins before Month 0 and resumes from its latest completed Snapshot. Snapshot does not duplicate effective settings; see ADR-50.

## Context

The simulation uses ordinal months rather than calendar dates. Persisting a separate starting Month would duplicate a fixed invariant, while a resumed run can derive its next Month from its latest Snapshot. Context also needs to retain the immutable starting settings, current settings, and simulation-level random root.

## Decision

- `Context` has three fields: `initial_settings`, `latest_settings`, and `root_key`.
- `initial_settings` and `latest_settings` use the same `UserSettings` value type, with `latest_settings` optional. Initial Context has `latest_settings = None`, so initial settings are effective. An accepted update creates a new Context value with `latest_settings = Some(updated)`, preserving initial settings and root key.
- A settings update supplies a complete `UserSettings` value. `Context::with_latest_settings` replaces `latest_settings` with that value and returns a new Context, while rejecting any change to `InitialValues` and preserving the original settings, simulation name, and root key. No field-level change or merge type is part of this contract.
- Context does not store a starting Month. Every new simulation begins at `Month(0)`; opening Ledger facts precede that month. Continuation derives the next Month from the latest completed Snapshot.
- Context does not retain a separate user-command history. Application commands are use-case inputs; each completed Snapshot retains the effective inputs used for its Month.
- Context depends on the random-engine crate for `RandomRootKey`; random-engine does not depend on Context. `UserSettings` remains an explicitly incomplete placeholder until representative Slice settings are defined, and Context must not acquire a dependency on `slice` to hold them.

## Alternatives

- Store a `start_month` in Context: rejected because every simulation starts at Month 0 and subsequent periods derive from Snapshots.
- Mutate latest settings in place: not selected; updates produce a new Context value, making the immutable initial settings and stable root explicit.
- Apply field-level change commands: rejected for the current contract; a complete settings value is validated and persisted as the accepted current user configuration.
- Retain every command as Context history: not selected; commands remain use-case inputs. Snapshot settings persistence was later removed by ADR-50.

## Consequences

- The Context shape is stable, while the concrete settings schema remains open. Each accepted full settings update replaces the previous latest value as one unit.
- Snapshot is the source for monthly progress and effective per-month inputs.
- `Month::first()` initializes a new simulation; no additional simulation-start field or ID is needed.
