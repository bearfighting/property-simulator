# ADR-14: Mortgage interest and payment convention

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md), [roadmap](../implementation-roadmap.md)
- **Test vectors:** [M-01–M-11](../test-vectors.md#mortgage-vectors)

## Context
The mortgage milestone needs an unambiguous monthly convention and invalid-input boundary.

## Decision
For each active month, calculate monthly rate as nominal annual rate divided by 12 and interest on opening principal for the full month. At month end apply payment to interest first, then principal. Unpaid interest capitalizes; actual payment is capped at opening principal plus interest to prevent overpayment in the final period. Keep intermediate arithmetic in decimal fixed-point precision and round only at the final currency posting boundary, to the currency minor unit using positive half-up rounding. Carry rounded posted interest into the balance calculation. Zero rate and no-loan state are valid. Negative annual rates, non-positive opening principal for an active loan, and negative payment are structural input errors. New loans and term changes require a valid effective `YearMonth` and take effect on its first day; day-level effective dates are unsupported.

## Alternatives considered
- Round interest to cents before allocating payments: creates repeated intermediate rounding that differs from the adopted final posting boundary.
- Negative rates or day-level partial accrual/effective dates: deferred pending an explicit versioned convention.

## Consequences
Use exact decimal arithmetic, not binary floating point. Tests must assert month-by-month interest, payment split, capitalization, and closing balance. A zero-payment month capitalizes the full posted interest; this is not delinquency/default. See vectors for payment below/equal/above interest and final cap.
