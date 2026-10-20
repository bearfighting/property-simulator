# ADR-21: M3 run randomness, persistence, and composition

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [roadmap](../implementation-roadmap.md), [ADR-15](ADR-15-random-streams-and-branches.md), [ADR-16](ADR-16-configuration-and-run-control.md), [ADR-20](ADR-20-composable-slices-and-random-sampling.md)
- **Test vectors:** M3 tests cover keyed substream isolation, per-period random input independence, proposal composition, run stopping, and saved snapshot/settings boundaries.

## Context

The earlier M3 outline required recovery of an unfinished run with the same random stream, and described storing random continuation state. Discussion before M3 clarified that the product saves completed simulation snapshots and the latest user settings; it does not need to resume an unfinished simulation as the same trial. Random outcomes need not be unique, and replaying the same configuration is not required to reproduce or differ from a prior trajectory. The outline also treated loan proceeds as a special funds-allocation case, despite loans being ordinary Slice-driven opening facts and monthly effects.

## Decision

- Persist completed monthly snapshots/results and the latest user settings needed to continue or branch. Do not persist random seeds, generator state, or draws for an unfinished month as recovery state. Do not persist additional system-generated runtime values solely to enable replay; retain only what is part of the completed result contract.
- Do not promise recovery of an unfinished run as the same trial, exact replay from identical settings, or different results from identical settings. A run started from a saved completed month is a new run/trial. Preserve the saved completed history.
- At application startup, read a 32-byte root key from the operating system and keep it in memory. Failure to obtain system entropy prevents simulation startup; wall-clock time is not a fallback. Derive a fresh 32-byte run key for each simulation with BLAKE3 keyed hashing over the run's `trial_id` and a process-local run sequence. Do not persist the root key, run keys, or generator state.
- Use `ChaCha12Rng` as the PRNG. Derive a 32-byte key for each semantic substream with BLAKE3 keyed hashing. The keyed message uses an unambiguous canonical encoding of a versioned tuple: trial ID, random category, Slice ID, activity/entity ID, `YearMonth`, model version, and sample purpose/key. Each distinct sample must have a distinct purpose/key; a sampler may consume multiple values from its own substream. Substreams therefore do not depend on Slice execution order or draws consumed by unrelated Slices.
- Treat random inputs for different time nodes as independent. The model may still make an outcome depend on opening state or other explicit inputs; independence applies to the random draws, not necessarily to the modeled outcomes. Values may repeat by chance and uniqueness is not enforced.
- These choices define the implementation, not a replay contract. The product does not promise that a given seed and semantic identity will yield the same draw across runs, code versions, or model versions. Fixed seeds may be injected in tests and diagnostics.
- Normal run completion occurs only when a user-configured end boundary/termination condition is reached or the user issues a between-period stop command. If an end month and one or more conditions are configured, any one reached/satisfied ends the run. Loan payoff, sale, negative cash flow, or another domain event does not stop a run unless explicitly selected as a user termination condition; fatal failures remain errors, not normal completion.
- A loan is not a special run-control or funds-routing case. It is configured through opening facts and ordinary monthly Slice effects. User opening asset/liability values determine opening net worth/equity as the balancing residual under assets = liabilities + net worth; if the user explicitly supplies net worth, validate it against that amount. Opening facts are not fabricated as past transactions. A monthly loan effect is recorded through the ordinary coordinated Slice and Ledger path.
- Slice composition guarantees apply to merging independent, conflict-free proposals: the merge operation is associative with an identity, and is commutative for independent proposals. They do not require every Slice state transformation to commute with every other transformation. Explicit dependencies, conflicts, and stages remain governed by ADR-20.
- The first stochastic model in M3 is an implementation milestone, not a user-selectable model family or an architectural choice. Any suitable model may be implemented first, provided it follows the sampling/Slice boundary and composition contract.

## Alternatives considered

- Preserve an unfinished run's exact trial using checkpoints: rejected because same-trial recovery is not a product requirement; completed month snapshots are the continuation boundary.
- Guarantee that identical settings either always reproduce or always differ: rejected; neither behavior is promised.
- Prevent repeated random values: rejected because it would alter the modeled probability distribution.
- Route all new loan proceeds through a required special-purpose allocation: rejected because loan accounting follows the configured opening state and ordinary monthly Slice effects, like other domain activity.
- Require each Slice transformation itself to commute: rejected; ADR-20 defines the algebraic contract for combining independent proposals and handles causal dependencies explicitly.

## Consequences

M3 does not implement unfinished-run checkpoint recovery or RNG-state persistence. A crash can lose work in the unfinished month; continuation starts from the latest completed snapshot as a new run. Completed generated outcomes remain part of saved history. Storage schema must distinguish completed simulation data from latest user settings. Random key material remains ephemeral; future changes to the PRNG or stream-key schema require an explicit version change but do not require migration of stored RNG state.

## References

- [`getrandom`](https://docs.rs/getrandom/latest/getrandom/) provides cross-platform access to operating-system random sources.
- [`rand_chacha`](https://docs.rs/rand_chacha/latest/rand_chacha/) documents ChaCha12/ChaCha20 generators and OS-based seeding.
- [`blake3`](https://docs.rs/blake3/latest/blake3/) provides keyed hashing and key derivation.
