# ADR-17: Currency-tagged money and opaque identifiers

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [architecture](../architecture.md), [ADR-13](ADR-13-ledger-profile.md), [roadmap](../implementation-roadmap.md)
- **Test vectors:** Existing CAD ledger and mortgage vectors in [test-vectors.md](../test-vectors.md); ID behavior uses fixed test IDs and invariants.

## Context
Amounts need an explicit denomination for ledger safety and display. Domain identifiers need type safety without coupling the core to clocks, randomness, or persistence.

## Decision
Represent each `Money` value as a signed `i128` coefficient plus an explicit decimal scale (up to 28 digits), and an ISO 4217 currency. This gives a fixed, checked numeric range beyond plausible property values without claiming unlimited precision. Constructors validate scale and decimal parsing; arithmetic aligns scales using checked operations and reports overflow. Never clamp or wrap an amount. A ledger uses one currency and rejects mixed-currency postings and aggregation. Currency minor-unit rounding uses positive half-up at the posting boundary. Do not implement exchange rates or cross-currency accounting in the initial scope. Locale formatting belongs to presentation and never changes denomination.

Represent each identifier as a distinct opaque strong type (for example `AccountId` and `LedgerTransactionId`) over a fixed-width 128-bit payload. Keep readable `AccountCode` separate from identity. Domain constructors receive IDs from their caller; generation stays outside domain rules. Tests use fixed IDs. A transaction retry reuses the same transaction ID, and the unit-of-work coordinator rejects a duplicate commit. M5 defines only the in-memory core contract; any textual encoding needed by SQLite or export can be chosen at that Phase II boundary.

## Alternatives considered
- Untyped strings or one shared ID type: permits accidental cross-entity references.
- Generate IDs from time or random state inside domain calculations: couples deterministic rules to infrastructure and simulation randomness.
- Convert currencies in the ledger: out of scope and requires explicit exchange-rate and rounding rules.

## Consequences
The supported monetary range is explicit and finite. Values outside it or calculations that overflow produce a clear checked numeric error; they are not silently altered. Tests cover values larger than the former `Decimal` range and checked overflow at the `i128` boundary. Typed IDs remain independent of exchange and ID-generation infrastructure. Future currency conversion requires a separate design decision.
