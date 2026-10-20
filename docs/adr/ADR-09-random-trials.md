# ADR-09: Explicit random trials and resampling

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-15](ADR-15-random-streams-and-branches.md)
- **Test vectors:** M3 fixed-seed diagnostic fixtures; exact product trajectory is not invariant.

## Context
Random outputs must be auditable while allowing new branches to explore different outcomes.

## Decision
Record trial identity, active rules/models, and completed results. Re-simulation from a saved completed snapshot creates a new trial and may resample; exact repeated output is not a product requirement, and identical settings are not promised either to reproduce or to differ from a prior run. Fixed seeds may be supplied in tests/diagnostics; a system-generated run seed is not a user setting or required recovery state. See [ADR-21](ADR-21-m3-run-randomness-and-persistence.md).

## Alternatives considered
- Requiring identical reruns: would constrain ordinary branch semantics and conflate recovery with resimulation.

## Consequences
Unfinished-run checkpoint recovery is not required; continuation from the most recent completed snapshot is a new run. The M3 random source, PRNG, and semantic substream derivation are fixed by [ADR-21](ADR-21-m3-run-randomness-and-persistence.md): startup OS entropy, ChaCha12, and BLAKE3 keyed derivation. These choices do not create a replay guarantee.
