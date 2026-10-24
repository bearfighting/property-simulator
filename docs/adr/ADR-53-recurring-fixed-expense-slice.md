# ADR-53: Recurring fixed-expense Slice

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [ADR-41](ADR-41-application-available-slice-kinds.md), [ADR-49](ADR-49-slice-catalogue-owns-slice-contracts.md), [Domain model](../domain-model.md)

## Context

The initial catalogue listed PropertyTax and Utility as separate Slice kinds, but both represent recurring monthly costs that remain active throughout the simulation. The simulation does not need to distinguish whether a configured expense represents property tax, land tax, education surcharge, or utilities; UX may explain those examples.

## Decision

- Replace `PropertyTax` and `Utility` with one `FixedExpense` Slice kind.
- A fixed expense remains active from the simulation start until the user disables it.
- Configure each fixed expense with exactly one of: a fixed monthly `Money` amount, or an annual rate applied to the simulation's unchanged total property value.
- Treat the rate as an annual decimal ratio, consistent with loan annual-rate input. Calculate the monthly amount as property value multiplied by annual rate and divided by twelve, then round for posting according to the money rules.
- Do not encode the real-world expense name in Core. UX may describe representative expenses.

## Alternatives considered

- Keep property tax and utility as separate Slices: rejected because the current model gives them the same monthly, continuing behavior and does not need their real-world names.
- Support both a fixed amount and a property-value rate at once: rejected because the configured amount must have one unambiguous calculation basis.

## Consequences

- `SliceKind` and `SliceSetting` expose one `FixedExpense` configuration.
- Its typed input includes the unchanged property value, cash account, expense account, and month; its event reports the amount paid.
- The Slice does not require simulated usage data or random samples. A future variable-usage model would require a separate explicit decision.
- ADR-41's separate `PropertyTax` and `Utility` kinds are superseded.

## Verification examples

- For property value 300,000 and annual rate 0.012, the unrounded monthly expense is 300 in the same currency.
- A fixed monthly amount is used directly and is not combined with a property-value rate.
