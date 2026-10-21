# ADR-18: Sufficient state and single-period transition

- **Status:** Accepted for state sufficiency; application-flow details updated by [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md)
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [architecture](../architecture.md), [ADR-03](ADR-03-arbitrary-opening-state.md), [ADR-15](ADR-15-random-streams-and-branches.md), [ADR-16](ADR-16-configuration-and-run-control.md), [ADR-21](ADR-21-m3-run-randomness-and-persistence.md)
- **Test vectors:** Continuity tests compare a transition from a complete opening snapshot with the corresponding period in a longer run; history-independence tests vary audit history while keeping transition inputs equal.

## Context
Calculation rules need enough explicit input to calculate from an opening state without reading hidden event history or knowing a run's final month. This keeps each calculation independent of history storage and application flow.

## Decision
Define the domain calculation boundary as:

`complete opening state + frozen current-period input + explicit current-period random input -> period result + complete closing state`.

The output state contains every domain fact needed by a subsequent calculation, including continuing activity terms, relevant generated state, and ledger balances. If a rule genuinely depends on older history, identify the information from that history that affects its result and pass it as a defined input or derived summary. Rules must not implicitly scan prior events, postings, or snapshots to determine the current result. History storage and audit views are outside this calculation boundary.

Each calculation uses the current period as input but has no total-duration or continue/stop policy. This ADR does not prescribe how an application might call calculations repeatedly. Every Slice instance evaluates from the same opening state and frozen inputs; no instance reads another instance's result. Accepted results combine associatively and commutatively, and incompatible combinations are rejected at the applicable composition boundary (ADR-20). Randomness is explicit as input supplied by the caller (ADR-21).

This is a sufficient-state, Markov-style computational boundary, not a claim that the modeled real-world process is statistically Markovian or that its variables are independent.

## Alternatives considered
- Let each rule query the full historical event and ledger log: rejected because it hides required state and ties domain calculation to storage/history retention.
- Put total duration and stopping policy inside the calculation: rejected because they are not needed to calculate one period. How an application invokes subsequent calculations remains undecided.
- Require strict independence from all earlier information: rejected because relevant past information can affect future behavior; it must be represented explicitly in current state.

## Consequences
Any opening state must be complete for the enabled calculations. A caller may pass a returned state to a later calculation; this does not define snapshots, history ownership, or a runtime checkpoint. Tests establish state sufficiency and continuity, not reproduction from hidden history. Adding a history-dependent rule requires an explicit input-contract change and corresponding tests.
