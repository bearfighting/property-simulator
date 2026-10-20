# ADR-16: Configuration and run controls

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [roadmap M3.0](../implementation-roadmap.md#m30--运行接口契约), [domain model](../domain-model.md), [ADR-01](ADR-01-monthly-run-control.md), [ADR-04](ADR-04-scenario-and-overrides.md), [ADR-15](ADR-15-random-streams-and-branches.md), [ADR-21](ADR-21-m3-run-randomness-and-persistence.md)
- **Test vectors:** M3 lifecycle/configuration tests cover these semantics.

## Context
Initial facts, run bounds, and changing monthly assumptions have different lifetimes and provenance.

## Decision
Scenario contains opening facts, future assumptions, configured activity instances, enabled calculation rules/Slices, and explicit versions. Higher-level run controls may contain an inclusive start month, an optional end month, zero or more month-end termination conditions, and between-period continue/stop control. Any configured end month or termination condition ends the run when reached/satisfied (OR). The user may also issue a stop command between completed periods, including before a configured boundary is reached; if there is no configured boundary/condition, the user must decide between periods whether to continue or stop. These controls govern whether the caller invokes another single-period transition and are not inputs to Slice/domain calculations. Evaluate configured termination conditions after the period result and closing snapshot. Before a transition, accept user changes to previously configured parameters/assumptions, current known state, and activity instances; record their effective period and source, then freeze current-period inputs. Changes take effect from the selected period and do not rewrite already committed periods. A current-state adjustment that changes an economically accounted balance must be represented by a state Effect and balanced Ledger transaction. Editing a previously closed period creates a branch from its snapshot, starts a new trial as defined in ADR-15, and records changed inputs/effective period. Validate required fields at configuration/rule boundaries.

### M3 run-call contract

The backend run-control boundary is one invocation per caller command. Each invocation accepts the applicable configuration updates and exactly one `Continue` or `Stop` command, and returns at most one newly completed monthly result, the latest run status, and a reason when the run has stopped. The concrete Rust type layout is an implementation choice; this is a behavioral contract, not a prescribed API shape.

- `Stop` takes effect between months: it does not invoke the monthly transition or create a result/snapshot. Configuration updates accompanying `Stop` are not applied; callers must send them with a later `Continue` if they are intended to affect a month. The response retains the most recent completed month, if any, and reports user stop as the stop reason.
- On `Continue`, validate and apply updates effective for the next month, freeze that month's inputs, then invoke the existing single-period transition (`coordinate_month`). The run controller must not duplicate Slice or other domain calculation rules.
- After a successful transition, construct the monthly result and month-end snapshot, then evaluate the inclusive configured end month and all termination conditions. Reaching the end month or satisfying any condition stops the run (OR semantics). Otherwise return a running status so the caller can issue the next command.
- With no configured end month or termination conditions, a successful month leaves the run running; the caller decides at each month boundary whether to continue or stop.
- Invalid configuration updates are returned before the transition and do not produce a successful monthly result or snapshot. A fatal single-period failure is returned as simulation failure, never as normal stop; it produces no successful result or snapshot for that month. Failure recovery/persistence guarantees are outside this contract.
- A command sent to a run that is already normally stopped is rejected as a run-state validation error; further simulation requires a new run, created from the appropriate opening state or completed snapshot. A failure-terminated run likewise cannot continue in place; any continuation begins as a new run from its latest completed snapshot, when available.

One invocation therefore advances no more than one month. Stop, month-end evaluation, and error classification are run-control responsibilities; the single-period transition remains the sole owner of monthly domain coordination.

### M3.1 Rust boundary

`SimulationRun` stores the immutable run boundaries, next month, current opening state and frozen assumptions, active instances, update history, latest completed result/snapshot, and status. Its `call` method accepts one command and typed updates. Updates carry a source and must target exactly the next month. Each successful monthly result retains the accepted updates, including source and effective month, so a saved snapshot preserves their audit record. An opening Ledger may contain postings only before the run start. Assumption and condition updates are applied to candidates and validated before the run changes. An economically accounted state adjustment is represented as one `StateAdjustment` containing both the revised state and a balanced transaction for the effective month; the revised cash amount must equal the resulting Ledger cash-account balance. The completed result also records a run-input Effect with before/after state, source, effective month, and transaction reference; its Effect ID is derived from the caller-supplied transaction ID and checked against other monthly Effects.

The run invokes an injected `SingleMonthExecutor` with validated frozen input, the candidate opening Ledger, and active instances. Before committing a monthly transition, the executor performs deterministic, side-effect-free validation of executor-specific activity and composition constraints. Sample, proposal, and transaction preflight validation errors also leave the run retryable because the coordinator has not committed; a fatal diagnostic fails it. The executor then samples, validates typed samples, invokes Slices, and uses `coordinate_month`, returning the completed monthly result and new Ledger. Before committing the result as a snapshot, run control checks that its month, opening state, frozen input, active instances, closing cash, Ledger history, and complete posted transactions (including entries and source metadata) agree. Transaction comparison is independent of posting order and keyed by transaction identity. The first successful result establishes the run's trial/branch identity; later months must preserve it. Run control then applies end-month/condition checks. If the final representable `YearMonth` completes without another stop condition, the result is retained and the run stops with a calendar-limit reason because no next period can be represented. Persistence and random-stream allocation remain outside M3.1.

## Alternatives considered
- AND-combined stopping conditions: can continue after a configured safety/desired stop is met.
- Retroactively edit a committed month: destroys audit history and reproducibility.
- Defer executor-specific activity validation until monthly execution: would make a correctable configuration error fail the run.
- Treat the final representable month as a failed transition when no next month can be created: would discard a successfully completed period and its snapshot.

## Consequences
Run control, configuration, and monthly input snapshots are separate explicit records. Mid-month changes are not supported.
