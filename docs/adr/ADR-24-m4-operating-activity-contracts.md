# ADR-24: M4 operating activity contracts

- **Status:** Accepted domain rules; runtime composition is historical/test-only
- **Date:** 2026-10-20
- **Related:** [domain model](../domain-model.md), [roadmap](../implementation-roadmap.md), [ADR-17](ADR-17-money-and-identifiers.md), [ADR-20](ADR-20-composable-slices-and-random-sampling.md), [ADR-21](ADR-21-m3-run-randomness-and-persistence.md), [ADR-22](ADR-22-m3-run-inputs-snapshot-and-cashflow-contract.md)
- **Test vectors:** M4.2 operating activity vectors in `docs/test-vectors.md`

## Context

This ADR records activity semantics and regression vectors. References below to a monthly result, active run, or runtime composition describe the historical M4 implementation or the intended monthly contract; current production APIs expose independent sampling and Slice calculations and do not assemble a run/month result. See [ADR-27](ADR-27-current-month-runtime-and-database-owned-history.md) and [ADR-28](ADR-28-pre-m5-crate-call-boundary-review.md).

M4 added one deterministic and one stochastic operating activity to exercise independent Slice behavior beyond rent, repair, and loans. Their configuration, samples, state/resource access, accounting, and provenance are specified here and implemented as independent calculation modules. They preserve the accepted Slice algebra: accepted results combine commutatively and associatively, with an empty result as identity; incompatible outputs are rejected at the composition boundary.

## Decision

### Property tax schedule

- A property-tax activity instance has a stable activity ID, non-negative currency-tagged payment amount, cadence (`monthly`, `quarterly`, or `annual`), first due `YearMonth`, cash account, and property-tax expense account. The first due month anchors the schedule; later due months are 1, 3, or 12 months apart. Calculation periods before the first due month produce no payment. A calculation beginning after the anchor uses the configured schedule without fabricating earlier transactions.
- Every active property-tax instance may carry a typed deterministic marker (`PropertyTax` sample-kind/value pair) as input provenance in the monthly result. The marker is not random, contains no sampled value, and is ignored by the tax calculation; tax behavior uses only its configuration and the frozen period. If the monthly result contract retains active inputs, the marker applies to due and non-due months alike.
- The configured amount is the amount actually paid on each due month. Annual or quarterly amounts are not accrued or spread over intervening months. This first M4 rule does not apply growth or inflation; those remain future model inputs.
- A due amount that rounds to a positive posted amount proposes one event with stable event key `property_tax.payment` and one balanced transaction (debit property-tax expense, credit cash classified as operating). The transaction's output key is `payment`; its `source_event_key` is `property_tax.payment` for the same instance and period. The contribution references the transaction ID. A zero/rounded-zero amount or non-due month proposes no event, Effect, transaction, or contribution.
- The amount currency must match the Ledger currency; the cash account must be a non-contra cash-position asset and the property-tax account must be a non-contra expense account.
- Include the rounded due-payment amount as operating cash-flow support in the period's aggregate support bound with all other active contributors under ADR-22. Reject an unrepresentable bound before the monthly transition.

### Utility usage charge

- A utility activity instance has a stable activity ID, configured usage unit, non-negative maximum integer usage, non-negative currency-tagged fixed rate per usage unit, cash account, and utilities expense account. The unit label describes the instance's quantity and rate; no unit conversion is performed.
- Each active month consumes exactly one typed `utility.usage` sample, a non-negative integer in the configured unit and in the inclusive range `0..=maximum_usage`. The sample is allocated by the ADR-21 `occupancy/operations` semantic stream; the Slice consumes it and never owns or calls an RNG. The usage probability law/support and charge rule are model rules supplied with the calculation inputs; the sample records its input source rather than a rule implementation identity. Rule changes evolve with the application as a whole and have no separate model version. Compatible sampler implementations remain interchangeable under ADR-20. Whether M6 returns sampled values for diagnostics is part of its result contract; persisted random-generator state and exact replay are not required.
- The first implemented probability law is uniform over every integer in `0..=maximum_usage`, using an unbiased inclusive sampler. This is a simple initial model assumption, not a calibrated utility forecast; changing the law changes application behavior and is reflected by the application version.
- Compute `usage × rate` with exact, checked decimal fixed-point arithmetic (never binary floating point); the exact product must fit the supported `Money` range before it is rounded once at the posting boundary to the currency minor unit using positive half-up. Before sampling, validate that the exact product for maximum configured usage is representable, then round that maximum exactly as a posted charge would be. Include this rounded maximum absolute posted amount—not the unrounded product—in the period's aggregate operating cash-flow support bound with all other active contributors under ADR-22. Reject invalid per-activity or aggregate support at the pre-sampling input boundary. An out-of-range sample or a sample whose carried period, Slice, activity, or required purpose does not match is a proposal validation error before commit. An unexpected checked overflow after successful prevalidation is a fatal transition error under ADR-19, not a correctable input error.
- The utility sampling boundary requires a `ValidatedInput` whose complete active activity set and aggregate support were preflighted for the same period before sampling. The caller supplies the random source and identity inputs through the sampling interfaces; utility uses the `occupancy/operations` category and purpose `utility.usage` under ADR-21. `RandomStreams` in test support rejects a trial mismatch. Other categories are valid identities for other models. The allocated `ActivitySample` carries period, Slice, activity instance, and purpose for receiving-boundary validation. Trial/category are not revalidated from the sample object because those fields are not part of its typed provenance.
- A charge that rounds to a positive posted amount proposes one event with stable event key `utility.usage_charged` and one balanced transaction (debit utilities expense, credit cash classified as operating). The transaction's output key is `usage-charge`; its `source_event_key` is `utility.usage_charged` for the same instance and period. The contribution references the transaction ID. Zero usage or a rounded-zero charge proposes no event, Effect, transaction, or contribution.
- The rate currency must match the Ledger currency; the cash account must be a non-contra cash-position asset and the utilities account must be a non-contra expense account.

### Shared composition and traceability

- Both activities read only their own immutable configuration and the frozen period; utilities additionally read their own explicit random sample. The property-tax marker is retained for per-instance input provenance but is not read by its calculation. Neither activity reads opening world-state values or produces state Effects. Their only write resources are their declared expense and cash account resources; they do not read account balances to calculate charges.
- Every transaction and event retains the applicable calculation identity, period, Slice, and activity identity. Each positive posting has an event-to-transaction source link. Stable event keys are `property_tax.payment` and `utility.usage_charged`; corresponding transaction output keys are `payment` and `usage-charge`. Event and transaction output IDs are derived under the ADR-20 identity rules from their respective keys. Zero economic outputs have no event or transaction link.
- Different activity instances retain separate transactions and output identities even when they share the cash account. Ledger batch posting derives the combined balance and cash-flow projection. A duplicate instance/output identity is rejected; no last-write-wins behavior is permitted.
- No dependency graph, execution stage, shared-state reducer, generic Effect framework, or production monthly executor is introduced for these activities.

## Alternatives considered

- Spread annual or quarterly tax evenly across months: rejected because it would report cash payments that did not occur; only an explicitly modeled budget/monthly-payment schedule may do that later.
- Pass utility consumption or charges from another Slice: rejected because it introduces same-month inter-instance causality and breaks independent composition.
- Let the utility Slice sample internally: rejected because sampling belongs to the ADR-21 sampler boundary and would couple output to execution order.
- Add shared operating-state Effects or a generic reducer framework: rejected because neither selected activity changes world state.

## Consequences

The domain rules remain useful to the M5 Core modules and M6 monthly calculation contract. The former integrated M4 coordinator, run, and snapshot behavior is retained only in test support and is not a production capability.

The property-tax schedule and utility charge are implemented by independent calculation modules; their former combined M4 runtime path remains only in test support. Utility distribution selection and calibration remain model concerns, provided samples obey the declared support and provenance. Configurable tax escalation, variable/seasonal tariffs, fractional usage, accrual accounting, unit conversion, and utility dependence on occupancy/weather are outside the current activity scope and require explicit model decisions. The M4.2 contract introduced no combined monthly runtime API; M6 owns that integration boundary.
