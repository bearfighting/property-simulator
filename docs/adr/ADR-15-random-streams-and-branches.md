# ADR-15: Random streams and branches

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-09](ADR-09-random-trials.md), [ADR-12](ADR-12-independent-random-categories.md)
- **Test vectors:** M3 tests cover branch isolation, stream allocation, provenance, and random-input separation. Recovery semantics are superseded by [ADR-21](ADR-21-m3-run-randomness-and-persistence.md).

## Context
Resampling a branch and continuing from completed simulation history have different semantics. The former checkpoint requirement for unfinished-run recovery was revisited before M3; see ADR-21.

## Decision
Each run has a `trial_id` and explicit engine/ruleset/model versions. A branch from a month-end snapshot retains parent history, records parent and fork month plus input changes/effective month, and starts a new trial. Persist completed snapshots/results and latest user settings; unfinished-run same-trial checkpoint recovery is not required. Organize category streams as in ADR-12. Keep probability distributions separate from random-stream allocation. Read OS entropy at application startup and use ChaCha12 with BLAKE3-derived semantic substreams as specified in ADR-21. This implementation does not imply a product promise of reproducible reruns.

## Alternatives considered
- Reuse parent trial for a new branch: confuses new exploration with continuation.
- Treat PRNG and stream derivation as unspecified: superseded by ADR-21, which fixes ChaCha12 and BLAKE3 semantic-key derivation for M3.

## Consequences
The operating-system entropy source is read at application startup; ChaCha12 and BLAKE3 semantic-key substreams are fixed by ADR-21. Tests verify isolation and provenance, not equality between independently started trajectories. Checkpoint recovery and encoding are out of M3 scope per ADR-21.
