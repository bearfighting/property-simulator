# ADR-06: Separate events, effects, and ledger

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md)
- **Test vectors:** Ledger posting examples are in [test vectors](../test-vectors.md#ledger-vectors).

## Context
Auditing needs to distinguish business facts, state changes, and economic postings.

## Decision
Events record what happened; decisions record chosen responses; effects describe state changes; balanced ledger transactions record economic postings. Keep traceable links among them.

## Alternatives considered
- A single mixed activity record: loses distinct meaning and reconciliation.

## Consequences
One event may link multiple effects and postings; projections derive from committed records.

Within a Slice instance proposal, an Effect or transaction may reference an event by its stable output key. The coordinator resolves the key to the event ID and retains the optional source-event link on the identified output; posted transactions retain the same link. A missing key is an invalid proposal. Each Effect also names its target resource so coordination can distinguish writes to different entities.
