# ADR-30: M6 monthly integration inputs and atomic Ledger settlement

- **Status:** Accepted
- **Date:** 2026-10-21
- **Related:** ADR-18, ADR-19, ADR-20, ADR-27, ADR-29
- **Test vectors:** M6 composition vectors; verify caller Ledger is unchanged on failure, successful transactions balance, cash flow derives from posted Ledger entries, and source attribution survives projection.

## Context

M6 must combine independently callable Slice calculations into one complete monthly result. The existing Slice contract uses associated sample types, while a month can contain different Slice implementations. Settlement must also validate all proposed postings and derive cash-flow output without exposing a partially committed caller Ledger if any calculation or audit fails.

## Decision

- The monthly calculation receives the opening `Ledger` by shared reference. It builds and settles against a candidate Ledger value, including atomic batch posting. It never mutates the caller's Ledger.
- A successful monthly result contains the closing state, identified events/effects, the journal transactions for this month, and per-instance cash-flow contributions projected from the successfully settled candidate Ledger. It does not return a second hand-maintained cash-flow fact or a resident Ledger/run object. The caller uses the returned journal transactions to advance or persist Ledger state.
- M6 invokes Slice calculations through caller-supplied adapters that bind a concrete `SimulationSlice` implementation to its explicitly supplied, correctly typed sample and expose the common Slice output contract. M6 may accept heterogeneous adapters through a narrow object-safe boundary; it does not name or depend on concrete Slice implementations, allocate randomness, or retain adapters after the call.
- M6 validates and combines all outputs before returning success. Validation, transition, settlement, projection, or audit failure returns an error and no successful monthly result. Candidate Ledger changes are discarded on failure.
- This adapter is a static call-boundary mechanism for M6, not a dynamic plugin registry, DSL, execution scheduler, or inter-Slice dependency system.
- The production input/result types are `MonthlyInput` and `MonthlyResult`. The input borrows the opening state, frozen input, active activities, explicit samples, opening loan continuations, and opening Ledger. Loan state is a separate input because loan calculations do not implement `SimulationSlice`. The result carries the month, opening and closing state (including both opening and closing loan continuations), activity/sample provenance, identified proposals, this month's posted journal delta, Ledger-derived per-transaction cash-flow contributions, and loan events/state effects.
- Loan calculations currently do not implement `SimulationSlice`. They remain part of the M6 contract through the typed loan continuation/event/effect fields and will be adapted at the M6 call boundary; the monthly result does not flatten them into untyped events or effects.
- M6.1 defines the input/result types and result structural validation. The result's internal constructor checks month/currency agreement, unique identities, source links, proposal-to-journal correspondence, cash-flow values against journal entries, and loan state effects against opening/closing continuation values. Collections are sorted by stable identity so caller input order does not affect result equality. It is not caller-constructible as a success value.
- M6.2 provides `BoundSliceAdapter` and an object-safe `MonthlySliceAdapter` boundary. The bound adapter retains the concrete Slice and typed sample, while the monthly call validates the opening context and activity set, checks adapter instance/configuration/sample-kind and typed-sample provenance before erasing the sample, checks one matching adapter and sample per non-loan activity, and checks one correctly sourced, currency-compatible continuation per loan activity. Ordinary Slice adapters are invoked against the same `ValidatedInput`. Results are returned in activity identity order; this ordering is for stable collection only and does not create domain execution dependencies. The caller-supplied `LedgerRead` must describe the borrowed opening Ledger; this relationship is a caller contract because the narrow interface does not expose Ledger identity. Repair support is passed as `RepairCostDistribution`; random sampling remains outside M6.
- M6.2 returns independent `SliceOutput` values only. M6.3 composes ordinary Slice outputs: it assigns IDs with the caller's `OutputIdFactory`, validates each output against its resource declaration, and merges compatible proposals. Loan calculation adaptation, Ledger settlement, projection construction, and final result assembly remain later M6 work.
- M6.4 exposes `calculate_monthly_result` as the complete production single-month entry point. The caller supplies ordinary Slice adapters, one `LoanSlice` for each loan activity, and a `LoanCalculation`; each loan configuration, activity, and opening continuation must match by activity and Slice identity. All activity identities, loan account roles, and opening liability balances are checked before ordinary Slice adapters are evaluated. Ordinary Slice and loan transactions are identified and settled together in one `post_batch` against a clone of the opening Ledger. Effects determine the closing property condition, while loan events/effects and next continuations remain strongly typed and are audited against both continuations and candidate Ledger liability balances. Only transactions in this month's proposal are projected from the candidate Ledger, so earlier postings in the same month are excluded. Closing cash is read from the candidate Ledger's configured cash account. Settlement, closing-state, and result-construction failures carry fatal phase diagnostics; the caller's Ledger remains unchanged on every failure. The current boundary calculates existing loans only: no month-start term updates, random sampling, persistence, or cross-month orchestration.

## Alternatives considered

- Mutate the caller's Ledger while calculating: rejected because later failure could expose partial month settlement.
- Have the application precompute every Slice and pass only proposals to M6: rejected because the monthly integration boundary is responsible for invoking independent calculations through their interfaces.
- Make M6 depend on each concrete Slice type: rejected because it reverses the dependency boundary and makes adding an independent Slice require changing integration code.
- Add a general plugin or scheduling framework: rejected because heterogeneous calls can be adapted at the M6 boundary without dynamic registration or ordering semantics.
- Author cash-flow values separately from Ledger transactions: rejected because cash flow must remain a Ledger-derived projection.

## Consequences

- M6 can test real heterogeneous Slice integration without IO and without changing the existing Core implementations.
- The candidate Ledger is the authority for transaction validation and cash-flow projection; only its successful journal delta is returned to the caller.
- Ordinary Slice and loan postings share one atomic monthly batch. Loan continuations reconcile to their individually configured liability accounts, and each successful result returns the updated continuation values for the caller's next invocation.
- The former coordinator `MonthlyResult` fixture is test-only and named `TestMonthlyResult`; trial/branch identity and historical update fields do not enter the production result contract.
- M6 tests must assert failure atomicity against the opening Ledger as well as result and Ledger reconciliation on success.
- The exact Rust input/result structs and adapter ergonomics are implementation details of M6.1/M6.2, provided they preserve this contract.
