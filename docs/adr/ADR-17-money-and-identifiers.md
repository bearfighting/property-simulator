# ADR-17: Currency-tagged money and opaque identifiers

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [architecture](../architecture.md), [ADR-13](ADR-13-ledger-profile.md), [roadmap](../implementation-roadmap.md)
- **Test vectors:** Existing CAD ledger and mortgage vectors in [test-vectors.md](../test-vectors.md); ID behavior uses fixed test IDs and invariants.

## Context
Amounts need an explicit denomination for ledger safety and display. Domain identifiers need type safety without coupling the core to clocks, randomness, or persistence.

## Decision
Represent each `Money` value with an exact decimal amount and explicit ISO 4217 currency code/minor-unit definition (for example, CAD with two minor-unit digits). A simulation ledger uses exactly one currency; reject mixed-currency postings and aggregation. Do not implement exchange rates, conversion, or cross-currency accounting in the initial scope. The core preserves the currency code and a canonical decimal representation. Locale, grouping separators, and symbol placement belong to the GUI/presentation layer; formatting never changes denomination.

Represent each identifier as a distinct opaque strong type (for example `AccountId` and `LedgerTransactionId`) over a fixed-width 128-bit payload. Keep readable `AccountCode` separate from identity. Domain constructors receive IDs from their caller; generation stays outside domain rules. Tests use fixed IDs. A transaction retry reuses the same transaction ID, and the unit-of-work coordinator rejects a duplicate commit. The precise generator and serialized textual encoding can be selected when application/infrastructure and the M5 data contract arrive.

## Alternatives considered
- Untyped strings or one shared ID type: permits accidental cross-entity references.
- Generate IDs from time or random state inside domain calculations: couples deterministic rules to infrastructure and simulation randomness.
- Convert currencies in the ledger: out of scope and requires explicit exchange-rate and rounding rules.

## Consequences
M1 can implement currency-aware exact amounts and typed IDs without an exchange subsystem or ID-generation dependency. Tests remain deterministic. Future display formatting does not affect ledger values; a future currency-conversion feature requires a separate versioned decision.
