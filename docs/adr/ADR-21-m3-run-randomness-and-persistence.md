# ADR-21: Caller-provided random inputs and sampling boundaries

- **Status:** Sampling contract retained; historical M3 run, persistence, and composition workflow superseded by [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md) and [ADR-29](ADR-29-layered-calculation-and-application-boundaries.md)
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [roadmap](../implementation-roadmap.md), [ADR-15](ADR-15-random-streams-and-branches.md), [ADR-16](ADR-16-configuration-and-run-control.md), [ADR-20](ADR-20-composable-slices-and-random-sampling.md), [ADR-26](ADR-26-scenario-scoped-runtime-and-concurrency.md)
- **Test vectors:** Historical M3 fixtures cover keyed substream isolation, proposal composition, run stopping, and snapshot/settings behavior; current production sampling accepts caller-provided sources.

## Context

The earlier M3 implementation combined stream allocation, run lifecycle, result history, and storage assumptions inside the simulation crate. The current core has no resident run state or persistence boundary. Sampling receives a caller-provided random source; allocation of that source belongs to the caller. Random outcomes need not be unique, and replaying the same configuration is not required to reproduce or differ from a prior trajectory.

## Decision

- Core calculation modules keep no run/session state. The caller passes a random source to sampling functions; the core does not allocate, retain, serialize, or restore streams.
- Do not promise recovery of unfinished work, exact replay from identical settings, or different results from identical settings. Persistence and history handling are outside this decision and are deferred until the application/storage design.
- A caller may provide a 32-byte root key obtained from the operating system; entropy failure has no clock-based fallback. The current randomness adapter derives a key as a pure function of the root key and explicit `trial_id` with BLAKE3 keyed hashing. Here `trial_id` is only a sampling-stream identity; it does not require a persisted trial, run entity, or branch workflow. Do not persist the root key, derived keys, or generator state. The randomness crate does not allocate hidden run sequence numbers.
- Use `ChaCha12Rng` as the PRNG. Derive a 32-byte key for each semantic substream with BLAKE3 keyed hashing. The keyed message uses an unambiguous canonical encoding of the tuple: trial ID, random category, Slice ID, activity/entity ID, `YearMonth`, and sample purpose/key. Each distinct sample must have a distinct purpose/key; a sampler may consume multiple values from its own substream. Substreams therefore do not depend on Slice execution order or draws consumed by unrelated Slices.
- Treat random inputs for different time nodes as independent. The model may still make an outcome depend on opening state or other explicit inputs; independence applies to the random draws, not necessarily to the modeled outcomes. Values may repeat by chance and uniqueness is not enforced.
- These choices define the implementation, not a replay contract. The product does not promise that a given seed and semantic identity will yield the same draw across runs or application versions. Fixed seeds may be injected in tests and diagnostics.
- This ADR does not define run completion, stop conditions, or multi-month execution. Those belong to a future caller of the monthly calculation boundary.
- A loan is not a special run-control or funds-routing case. It is configured through opening facts and ordinary monthly Slice proposals. User opening asset/liability values determine opening net worth/equity as the balancing residual under assets = liabilities + net worth; if the user explicitly supplies net worth, validate it against that amount. Opening facts are not fabricated as past transactions. A loan Slice returns a proposal; a future application boundary may compose it with other outputs and accounting effects.
- Every active Slice instance computes independently from the same opening state, frozen inputs, and explicit samples. Accepted instance results combine associatively and commutatively; dependencies, stages, and cross-instance result reads are not part of the current design. Incompatible results are rejected before commit, as specified by ADR-20.
- The first stochastic model in M3 is an implementation milestone, not a user-selectable model family or an architectural choice. Any suitable model may be implemented first, provided it follows the sampling/Slice boundary and composition contract.

## Alternatives considered

- Preserve an unfinished run's exact trial using checkpoints: rejected because same-trial recovery is not a product requirement. The current calculation interface does not define snapshot continuation or recovery.
- Guarantee that identical settings either always reproduce or always differ: rejected; neither behavior is promised.
- Prevent repeated random values: rejected because it would alter the modeled probability distribution.
- Route all new loan proceeds through a required special-purpose allocation: rejected because loan accounting follows the configured opening state and ordinary monthly Slice effects, like other domain activity.
- Allow same-month causal dependencies between Slice instances: rejected; they would invalidate result permutation and grouping invariance.

## Consequences

M3 does not implement unfinished-run checkpoint recovery or RNG-state persistence. Failure handling, snapshot continuation, and persistence are outside the current calculation interface and remain for a future caller/application design. Generated outcomes and retained user settings belong to application storage, not a core-owned scenario history. Random key material remains ephemeral. A change to the PRNG or stream derivation changes the application as a whole; it does not create a separately versioned random-stream contract or stored-state migration.

## Previous M3.4 implementation notes (historical)

The earlier implementation used an atomic process sequence and injected stream allocation through `SimulationRun`. Run control and stream allocation are now test-only or caller-side; the current contract above supersedes that composition.

`property-simulation-randomness` is the random-source adapter crate. `RootKey::from_os()` obtains exactly 32 bytes via `getrandom`; entropy failure returns `RandomnessError::EntropyUnavailable`, with no fallback. Tests/diagnostics may construct `RootKey::from_seed`. `RunKeyFactory::run_streams(trial)` is stateless: a run key is BLAKE3 keyed hash over the domain tag `property-simulation/run-key\0` and the explicit 16 trial-ID bytes. Repeated calls with the same root and trial produce the same stream identity; callers needing distinct trials supply distinct trial IDs. No process sequence is retained.

Semantic streams use ChaCha12 seeded by BLAKE3 keyed hash under the run key. The message is the tag `property-simulation/semantic-stream\0`, then fixed-width fields in order: trial ID (16 bytes), category (`u8`: environment=1, property=2, policy/macro=3, occupancy/operations=4), Slice ID (16), activity instance ID (16), year (`u32` little-endian), month (`u8`), and purpose length (`u64` little-endian) followed by UTF-8 purpose bytes. This encoding is unambiguous. A stream collection is bound to its run's trial and rejects a mismatched trial identity. Every separate draw purpose gets its own stream identity; a sampler can consume multiple words from its stream.

The earlier monthly executor exposed an optional stream-allocation hook, and `SimulationRun` allocated a run key. These APIs now exist only in the test fixture. Production sampling accepts caller-provided `RandomSource` and `Sampler` interfaces; it does not depend on trial/branch lifecycle. The stream encoding and fixed-key diagnostics do not promise identical draws across application versions or unique values.

## References

- [`getrandom`](https://docs.rs/getrandom/latest/getrandom/) provides cross-platform access to operating-system random sources.
- [`rand_chacha`](https://docs.rs/rand_chacha/latest/rand_chacha/) documents ChaCha12/ChaCha20 generators and OS-based seeding.
- [`blake3`](https://docs.rs/blake3/latest/blake3/) provides keyed hashing and key derivation.
