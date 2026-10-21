# ADR-20: Composable slices and decoupled random sampling

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [ADR-05](ADR-05-slice-coordination.md), [ADR-06](ADR-06-events-effects-ledger.md), [ADR-09](ADR-09-random-trials.md), [ADR-12](ADR-12-independent-random-categories.md), [ADR-17](ADR-17-money-and-identifiers.md), [ADR-18](ADR-18-sufficient-state-transition.md), [ADR-19](ADR-19-validation-only-domain-errors.md)
- **Test vectors:** Independent proposal merging must be invariant under permutation and partitioning; sampled values are supplied explicitly to Slice calculations; repair probability and minor-unit cost support are exact and bounded as specified below and in [test-vectors.md](../test-vectors.md#m2-monthly-activity-vectors).

## Context

The monthly-result and composition text in this ADR records the intended calculation contract and historical test coverage. The current production API has no monthly result assembler, run object, or coordinator; that boundary is being defined in M5. See ADR-27 and ADR-28.

M2 needs to support independent activity calculations, parallel or grouped evaluation, traceable outputs, and experiments with different random sampling methods. Coupling a Slice to a particular sampler would make those experiments invasive. Runtime failures also need a clear boundary between expected input validation and unrecoverable implementation or infrastructure faults.

## Decision

Every active Slice instance evaluates independently against the same immutable opening state, frozen period inputs, and its explicit samples. A Slice cannot read another instance's result, and the system does not define dependencies, ordered stages, or same-month causal handoffs between Slice instances. The operation that combines accepted instance results is associative and commutative and has an identity: reordering, regrouping, or parallel evaluation cannot change the combined result. A combination that cannot obey these laws is rejected at the composition boundary before commit. State writes with identical values may be idempotently combined; conflicting values are rejected. Ledger postings remain distinct and are combined as a batch whose validity is independent of input order.

Activities are configured as Slice instances with caller-assigned stable instance IDs. A Slice supplies output keys and proposal content; successful Slice evaluation returns an opaque output bound to the exact activity instance and frozen period that produced it. ID assignment accepts this bound output, so callers cannot relabel proposal provenance between evaluation and identification. `identify_proposal` uses an injected ID factory and the bound activity/period provenance to derive output IDs from caller-supplied identity and output keys. IDs must not depend on execution order or grouping. Re-evaluating the same instance with the same identity produces the same IDs. Pure merge functions use canonical stable-ID ordering.

The injected ID factory must be deterministic and collision-resistant. Since IDs have a fixed 128-bit representation and output keys are not length-bounded, mathematical injectivity over all possible inputs is not promised. `merge_proposals` rejects duplicate IDs within each output kind. Callers are responsible for using identity from one calculation consistently; the current core has no run object that enforces trial/branch isolation.

Probability distributions, sampling algorithms, and Slices are separate components. A distribution defines the supported values and their probability law; a sampler implements a method for producing a value from that distribution; a Slice consumes typed, explicit inputs and does not call or own a random-number generator. Stochastic inputs are sampled values. A deterministic Slice may instead carry a typed unit marker as input provenance; the marker is not random and is ignored by the calculation. If a future monthly result retains active input records, it can preserve one typed record per instance. Sample provenance (instance, Slice, sample kind, and period) is checked against the receiving Slice and frozen period. Different compatible samplers may be paired with the same distribution, and the same Slice may consume values produced by different samplers. Compatibility is validated where the caller combines these components. The distribution's full support must fit validated numeric and domain constraints; unbounded support requires explicit truncation/value semantics or rejection of that configuration. Sampling receives explicit interfaces; stream allocation remains caller-owned.

For the M2 repair example, occurrence probability is an exact rational `numerator / denominator` with a nonzero denominator and numerator no greater than denominator. Conditional repair cost is uniform over integer minor-unit amounts from zero through the floored maximum. The caller injects a `RandomSource`; samplers do not allocate or derive streams. Random-source failures are returned as sampling errors. Distribution compatibility is checked before sampling, while the existing Slice input boundary remains responsible for matching activity instance, sample kind, frozen period, and support.

Unexpected runtime errors, including checked numeric overflow outside validated supported bounds, are fatal calculation failures and are not user-correctable domain outcomes. The calling layer must treat fatal diagnostics as a stopped calculation; the current core has no run lifecycle. A failed calculation does not produce a successful monthly result. Failed-run recovery/retry and external persistence guarantees are outside this contract (ADR-27).

## Alternatives considered

- Allow a Slice to consume another Slice's same-month result: rejected because it makes the result depend on evaluation order and grouping, defeating the composition contract.
- Let a Slice call a sampler directly: rejected because it couples domain rules to sampling algorithms and prevents clean sampler experiments.
- Treat fatal runtime failures as recoverable user input errors: rejected because it misattributes implementation or infrastructure failures.
- Guarantee restartable, transactionally consistent persistence after every calculation failure: rejected as a current requirement. Calculation functions return errors; recovery of a failed run and external persistence consistency are not part of the current boundary.

## Consequences

M2 defines explicit proposal merge rules and stable provenance/ID derivation. It tests independent outputs under reordered and partitioned evaluation, while testing conflict rules separately. Slice tests use explicit typed samples; distribution and sampler tests are independent. Full monthly assembly and run stopping are not production APIs in the current pre-M5 boundary.

## M2.4 resource declarations and merge rules (2026-10-19)

The implementation details in the M2.4/M2.5 notes below describe the earlier coordinated path and its regression tests. They do not imply a production coordinator; current module boundaries are recorded in ADR-28.

Each active Slice instance derives typed `ResourceKey` reads and writes from its frozen input. These declarations validate output provenance and declared writes; they do not create dependencies or ordering. Read/write overlap is valid because every instance reads the same opening state. Effects and journal account postings must target declared writes; account resources use the canonical `account:<32 lowercase hex digits>` key. Shared state writes combine only when their resulting values agree; differing values are rejected before commit.

Cash changes remain separate traceable double-entry transactions and account balances are derived by the Ledger. State effects on one resource merge idempotently when their resulting values are equal; differing values are rejected. Output IDs remain unique per output kind and merged collections retain canonical ID order. This M2.4 framework does not implement complete monthly coordination.

Alternatives considered: closed resource enums were rejected in favor of extensible typed keys; causal ordering and implicit conflict ordering were rejected because they violate permutation/grouping invariance; netting cash postings was rejected because it loses activity provenance.

Consequences and vectors: empty declarations form a valid isolated composition; accepted result composition is invariant under permutation and regrouping; duplicate instances, provenance mismatches, undeclared output writes, and incompatible shared state writes fail before commit; read/write overlap does not imply ordering; distinct transactions remain individually traceable.

## M2.5 single-period coordination (2026-10-19)

`propose_slice` binds each output to the opening state, frozen input, resource declaration, and sample used for that evaluation. The production API exposes output identification, proposal merging, and resource validation as separate functions; it does not provide a monthly coordinator or Ledger commit path. The former `coordinate_month` behavior is retained only in test support and is not a production contract. No instance output is supplied as another instance's input. M5 will define the full monthly input/output boundary after the current crate-boundary refactor.

Alternatives considered: mutating the caller Ledger was rejected because result construction or state reduction could then fail after partial commit; allowing callers to separately assemble resource declarations was rejected because it permits provenance drift. A general Effect engine and automatic Slice dispatch remain out of scope.

Consequences and vectors: output permutation and regrouping do not change the combined result; rent and repair in one month retain separate source-linked balanced postings; closing cash matches the Ledger balance; a repair Effect updates condition while no repair preserves opening condition; invalid samples, unsupported Effects, incompatible writes, and transaction preflight failures retain validation errors with transaction keys where available; failures during an accepted Slice transition, post-preflight ledger commit, closing-state derivation, or result construction are fatal diagnostics with phase/month and available identity. The coordinator returns no partial result on failure. Fatal failure stops the simulation; external persistence rollback/recovery is not promised.
