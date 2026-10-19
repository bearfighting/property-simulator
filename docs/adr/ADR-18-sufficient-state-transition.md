# ADR-18: Sufficient state and single-period transition

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [architecture](../architecture.md), [ADR-03](ADR-03-arbitrary-opening-state.md), [ADR-15](ADR-15-random-streams-and-branches.md), [ADR-16](ADR-16-configuration-and-run-control.md)
- **Test vectors:** Continuity tests compare a transition from a complete opening snapshot with the corresponding period in a longer run; history-independence tests vary audit history while keeping transition inputs equal.

## Context
The domain needs to continue from arbitrary valid snapshots and support audit history, random trials, and user-controlled runs. If rules read the whole event log or know a run's final month, computation becomes coupled to persistence and run control; it also becomes unclear which opening states are sufficient to continue.

## Decision
Define the domain calculation boundary as:

`complete opening state + frozen current-period input + explicit current-period random input -> period result + complete closing state`.

The closing state contains every fact needed for the next transition, including continuing activity terms, relevant generated state, ledger balances, and any explicit random-process continuation state. If a rule genuinely depends on older history, identify the information from that history that affects its result and carry it forward as a defined state field or derived summary. Rules must not implicitly scan prior events, postings, or snapshots to determine the current result. History remains available for audit, display, and projections.

The transition uses the current period as data but has no total-duration or continue/stop policy. A higher-level run controller repeatedly invokes the same transition and applies end-month, stop-condition, user-stop, or cancellation policy between completed periods. Same-period causal dependencies are passed through explicit stages/results in the transition context. Randomness is likewise explicit as current-period samples or checkpointed stream state; it is not hidden global state.

This is a sufficient-state, Markov-style computational boundary, not a claim that the modeled real-world process is statistically Markovian or that its variables are independent.

## Alternatives considered
- Let each rule query the full historical event and ledger log: rejected because it hides required state and ties domain calculation to storage/history retention.
- Put total duration and stopping policy inside the transition: rejected because those are caller/run-control concerns and the same transition should serve bounded, stepwise, and resumed runs.
- Require strict independence from all earlier information: rejected because relevant past information can affect future behavior; it must be represented explicitly in current state.

## Consequences
Any valid opening snapshot must be complete for the enabled rules. Snapshots and checkpoints preserve continuation state, while audit history can be independently retained or projected. Tests should establish state sufficiency and continuity, not require the transition to reproduce outputs from hidden history. Adding a history-dependent rule requires an explicit state-contract change and corresponding version/tests.
