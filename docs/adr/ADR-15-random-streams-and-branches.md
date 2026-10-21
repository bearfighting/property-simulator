# ADR-15: Random streams and branches

- **Status:** Sampling semantics retained; branch workflow superseded by [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md) and [ADR-29](ADR-29-layered-calculation-and-application-boundaries.md)
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [ADR-09](ADR-09-random-trials.md), [ADR-12](ADR-12-independent-random-categories.md)
- **Test vectors:** Historical M3 tests cover branch isolation; sampling tests cover stream allocation, provenance, and random-input separation. Recovery and branch workflows are not current requirements.

## Context
Resampling a branch and continuing from completed simulation history have different semantics. The former checkpoint requirement for unfinished-run recovery was revisited before M3; see ADR-21.

## Decision
Random stream identities may include an explicit `trial_id`, category, Slice ID, activity ID, period, and purpose. The current calculation core does not create runs or branches and does not allocate or retain streams. The caller supplies a `RandomSource` to a sampling function; distribution rules remain separate from stream allocation. The randomness adapter can derive ChaCha12 semantic streams from caller-supplied root/trial identity using BLAKE3 as specified in ADR-21. A future application may choose where to obtain root entropy. Branching, snapshots, scenario management, and scheduling are deferred and are not current core contracts. No random/model-specific version fields are used; the application version remains the sole application version identifier. None of this promises reproducible reruns.

## Alternatives considered
- **Historical branch proposal:** reuse a parent trial for a new branch. Branch creation is deferred, so this is not a current product rule.
- Treat PRNG and stream derivation as unspecified: superseded by ADR-21, which records the current sampling adapter implementation; this does not guarantee replay across calls or application versions.

## Consequences
Sampling and stream allocation are separate calls. Tests verify isolation and provenance, not equality between independently started trajectories. Checkpoint recovery, branch behavior, and persistence are outside the current calculation scope.
