# ADR-09: Explicit random trials and resampling

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-15](ADR-15-random-streams-and-branches.md)
- **Test vectors:** M3 fixed-seed diagnostic fixtures; exact product trajectory is not invariant.

## Context
Random outputs must be auditable while allowing new branches to explore different outcomes.

## Decision
Record trial identity, optional seed, active rules/models, and random input provenance. Re-simulation from a snapshot creates a new trial and may resample; exact repeated output is not a product requirement.

## Alternatives considered
- Requiring identical reruns: would constrain ordinary branch semantics and conflate recovery with resimulation.

## Consequences
Checkpoint recovery continues the same trial; random algorithm/stream details are fixed separately in M3.
