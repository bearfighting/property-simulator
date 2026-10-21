# ADR-22: Monthly inputs and Ledger-derived cash flow

- **Status:** M3 implementation details retained; current calculation boundary is [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md)
- **Date:** 2026-10-20
- **Related:** [roadmap](../implementation-roadmap.md), [domain model](../domain-model.md), [ADR-13](ADR-13-ledger-profile.md), [ADR-14](ADR-14-mortgage-interest.md), [ADR-16](ADR-16-configuration-and-run-control.md), [ADR-20](ADR-20-composable-slices-and-random-sampling.md), [ADR-21](ADR-21-m3-run-randomness-and-persistence.md), [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md)
- **Test vectors:** M3 command-validation vectors are historical; Ledger posting, cash-flow projection, and per-instance source attribution remain relevant at their calculation boundaries.

## Context

M3 introduced a run-control API, monthly result/snapshot types, and per-Slice cash-flow contribution structures. Later clarification narrowed the current Core boundary to a monthly input/output calculation and established the Ledger as the source for cash-flow results. The M3 implementation details below that imply history/archive ownership or separately authored cash-flow facts are superseded by ADR-27. ADR-29 assigns independent module work to M5 and complete single-month input/output composition to M6.

## Current contract

- Each monthly calculation receives explicit opening state and Ledger, validated current-period inputs, and the random inputs required by the active rules. It returns a complete monthly result containing the closing state and Ledger changes, or an error.
- A successful result includes events, state changes, Ledger postings, and the month's cash-flow result. The cash-flow result is derived from posted Ledger cash entries and their categories; Ledger transaction source metadata attributes the entries to an activity instance. The result may present this projection as a per-instance monthly sequence, but it is not a second independently generated accounting record.
- The next month is calculated by passing the previous month's closing state and Ledger as the next opening inputs. Core does not retain complete run history, perform persistence, or manage branches as part of this contract.
- User-correctable input errors are returned before calculation. A fatal calculation error stops the simulation and produces no successful result for that month; no recovery or retry of the failed run is required.
- Loan calculations follow ADR-14 and post balanced Ledger transactions. Cash-flow classification and amounts are projected from those entries; loan principal is financing cash flow and paid interest is operating cash flow.

## M3 implementation note

M3 historically contained additional archive/history and `CashFlowContribution` structures. Runtime composition and archive implementations now live only in test support; they do not expand the current production API. M6 is responsible for defining a monthly input/output function that follows ADR-27 and this Ledger-derived cash-flow boundary. A later Application layer will call Data Access to persist user inputs, the complete monthly result/snapshot data (including its closing state and per-instance Ledger-derived cash-flow sequence), and Ledger records. Ledger changes in the result and the persisted Ledger records are the same economic facts, not a second ledger. SQLite layout remains for that later stage.
