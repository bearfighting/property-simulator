# ADR-02: One monthly transactional tick

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [architecture](../architecture.md), [domain model](../domain-model.md)
- **Test vectors:** M6 month-boundary vectors belong to the M6 checklist; M3 lifecycle coverage is historical test-support only.

## Context
Slice calculations need a stable shared period for proposals and postings. Earlier designs also treated a month as a run-control tick that produced a retained snapshot; current scope assigns single-month integration to M6 and leaves repeated calls and persistence to later Application/Data Access work.

## Decision
One calendar month is the period of one calculation. M6 integrates that period and returns one complete monthly result containing its closing state and Ledger changes; it does not retain a run or snapshot history. A 20-year example is 240 successive monthly calculations, not a duration limit.

## Alternatives considered
- Daily or variable tick sizes: unnecessary for the first monthly model.

## Consequences
Sub-month accrual is out of scope. A later Application may pass one calculation's closing state to the next call; the calculation modules do not own that sequence.
