# ADR-15: Random streams, branches, and recovery

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-09](ADR-09-random-trials.md), [ADR-12](ADR-12-independent-random-categories.md)
- **Test vectors:** M3 tests cover branch isolation, stream allocation, provenance, and recovery without duplicate commits.

## Context
Resampling a branch and resuming an interrupted run have different semantics.

## Decision
Each run has a `trial_id`, optional seed, and explicit engine/ruleset/model versions. A branch from a month-end snapshot retains parent history, records parent and fork month plus input changes/effective month, and starts a new trial. A checkpoint resumes the same trial and preserves sufficient stream state to avoid duplicate commits. Organize category streams as in ADR-12. Keep probability distributions separate from random-stream allocation.

## Alternatives considered
- Reuse parent trial for a new branch: confuses new exploration with continuation.
- Fix a specific PRNG or derivation now: implementation belongs to M3.

## Consequences
PRNG, stream derivation, and checkpoint encoding remain explicit M3 decisions. Tests verify isolation and recovery invariants, not equality between independently branched trajectories.
