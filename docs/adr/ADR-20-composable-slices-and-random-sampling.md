# ADR-20: Composable slices and decoupled random sampling

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [ADR-05](ADR-05-slice-coordination.md), [ADR-06](ADR-06-events-effects-ledger.md), [ADR-09](ADR-09-random-trials.md), [ADR-12](ADR-12-independent-random-categories.md), [ADR-17](ADR-17-money-and-identifiers.md), [ADR-18](ADR-18-sufficient-state-transition.md), [ADR-19](ADR-19-validation-only-domain-errors.md)
- **Test vectors:** Independent proposal merging must be invariant under permutation and partitioning; sampled values are supplied explicitly to Slice calculations; repair probability and minor-unit cost support are exact and bounded as specified below and in [test-vectors.md](../test-vectors.md#m2-monthly-activity-vectors).

## Context

M2 needs to support independent activity calculations, parallel or grouped evaluation, traceable outputs, and experiments with different random sampling methods. Coupling a Slice to a particular sampler would make those experiments invasive. Runtime failures also need a clear boundary between expected input validation and unrecoverable implementation or infrastructure faults.

## Decision

Every active Slice instance evaluates independently against the same immutable opening state, frozen period inputs, and its explicit samples. A Slice cannot read another instance's result, and the system does not define dependencies, ordered stages, or same-month causal handoffs between Slice instances. The operation that combines accepted instance results is associative and commutative and has an identity: reordering, regrouping, or parallel evaluation cannot change the combined result. A combination that cannot obey these laws is rejected at the composition boundary before commit. State writes with identical values may be idempotently combined; conflicting values are rejected. Ledger postings remain distinct and are combined as a batch whose validity is independent of input order.

Activities are configured as Slice instances with caller-assigned stable instance IDs. A Slice supplies output keys and proposal content; successful Slice evaluation returns an opaque output bound to the exact activity instance and frozen period that produced it. ID assignment accepts this bound output, so callers cannot relabel proposal provenance between evaluation and identification. The coordinator attaches the authoritative instance, Slice, model version, and period provenance, then derives stable output IDs from the trial/branch, period, instance ID, output kind, and output key. IDs must not depend on execution order, batching, or worker scheduling. Re-evaluating the same instance in the same trial and period produces the same output IDs. Combined output collections use canonical stable-ID ordering.

The injected ID factory must be deterministic and collision-resistant. Since IDs have a fixed 128-bit representation and output keys are not length-bounded, mathematical injectivity over all possible inputs is not promised. The monthly coordination boundary rejects duplicate IDs within each output kind, and a period result retains its trial and branch identity so outputs from different runs cannot be combined accidentally.

Probability distributions, sampling algorithms, and Slices are separate components. A distribution defines the supported values and their probability law; a sampler implements a method for producing a value from that distribution; a Slice consumes typed, explicit sampled values and does not call or own a random-number generator. Sample provenance (instance, Slice, model version, sample kind, and period) is checked against the receiving Slice and frozen period, and the period result checks the sample kind against the activity instance. Different compatible samplers may be paired with the same distribution, and the same Slice may consume values produced by different samplers. Compatibility is validated at composition boundaries. The distribution's full support must fit validated numeric and domain constraints; unbounded support requires explicit truncation/value semantics or rejection of that configuration. M2 may inject explicit samples; random-stream allocation remains in M3.

For the M2 repair example, occurrence probability is an exact rational `numerator / denominator` with a nonzero denominator and numerator no greater than denominator. Conditional repair cost is uniform over integer minor-unit amounts from zero through the floored maximum. The caller injects a `RandomSource`; samplers do not allocate or derive streams. Random-source failures are returned as sampling errors. Distribution compatibility is checked before sampling, while the existing Slice input boundary remains responsible for matching activity instance, sample kind, frozen period, and support.

Unexpected runtime errors, including checked numeric overflow outside validated supported bounds, are fatal simulation failures and are not user-correctable domain outcomes. The run stops and produces diagnostics for engineering investigation. The system does not promise recovery, rollback, or persistent-state consistency after such an unrecoverable failure. Successful period completion still requires the normal state, event, and ledger result to satisfy their invariants.

## Alternatives considered

- Allow a Slice to consume another Slice's same-month result: rejected because it makes the result depend on evaluation order and grouping, defeating the composition contract.
- Let a Slice call a sampler directly: rejected because it couples domain rules to sampling algorithms and prevents clean sampler experiments.
- Treat fatal runtime failures as recoverable user input errors: rejected because it misattributes implementation or infrastructure failures.
- Guarantee restartable, transactionally consistent persistence after every fatal runtime failure: deferred; recovery and persistence semantics belong to later storage/run-control work and are not promised for unrecoverable failures.

## Consequences

M2 defines explicit proposal merge rules and stable provenance/ID derivation. It tests independent results under reordered and partitioned evaluation, while testing conflict rules separately. Slice tests use explicit typed samples; distribution and sampler tests are independent, with compatibility tests at their composition boundary. Runtime failure handling stops the run and records diagnostics; no simulation result is emitted for a failed period.

## M2.4 resource declarations and merge rules (2026-10-19)

Each active Slice instance derives typed `ResourceKey` reads and writes from its frozen input. These declarations validate output provenance and declared writes; they do not create dependencies or ordering. Read/write overlap is valid because every instance reads the same opening state. Effects and journal account postings must target declared writes; account resources use the canonical `account:<32 lowercase hex digits>` key. Shared state writes combine only when their resulting values agree; differing values are rejected before commit.

Cash changes remain separate traceable double-entry transactions and account balances are derived by the Ledger. State effects on one resource merge idempotently when their resulting values are equal; differing values are rejected. Output IDs remain unique per output kind and merged collections retain canonical ID order. This M2.4 framework does not implement complete monthly coordination.

Alternatives considered: closed resource enums were rejected in favor of extensible typed keys; causal ordering and implicit conflict ordering were rejected because they violate permutation/grouping invariance; netting cash postings was rejected because it loses activity provenance.

Consequences and vectors: empty declarations form a valid isolated composition; accepted result composition is invariant under permutation and regrouping; duplicate instances, provenance mismatches, undeclared output writes, and incompatible shared state writes fail before commit; read/write overlap does not imply ordering; distinct transactions remain individually traceable.

## M2.5 single-period coordination (2026-10-19)

`propose_slice` binds each opaque output to the exact opening state, frozen input, resource declaration, and sample value used for that evaluation. `coordinate_month` accepts evaluated outputs rather than dispatching concrete Slices. It identifies outputs first, checks each output's instance/Slice/version provenance against its bound declaration, validates one source-matching typed sample per active instance and resource declarations, then merges proposals. No instance output is supplied as another instance's input. The typed sample records supplied for the result must equal those captured by the outputs. The validated input also retains its opening Ledger snapshot; coordination rejects a different Ledger. It reduces supported Effects and preflights all proposed transactions against a cloned Ledger before the submission step. Correctable transaction errors retain their proposal-validation report and transaction keys. It then applies the validated batch to a cloned Ledger, derives closing cash from that Ledger, and constructs the period result. Other Effect targets fail as invalid proposals before ledger submission. Only after all steps succeed does it return `MonthlyResult` and the new Ledger; caller state is never mutated.

Alternatives considered: mutating the caller Ledger was rejected because result construction or state reduction could then fail after partial commit; allowing callers to separately assemble resource declarations was rejected because it permits provenance drift. A general Effect engine and automatic Slice dispatch remain out of scope.

Consequences and vectors: output permutation and regrouping do not change the combined result; rent and repair in one month retain separate source-linked balanced postings; closing cash matches the Ledger balance; a repair Effect updates condition while no repair preserves opening condition; invalid samples, unsupported Effects, incompatible writes, and transaction preflight failures retain validation errors with transaction keys where available; failures during an accepted Slice transition, post-preflight ledger commit, closing-state derivation, or result construction are fatal diagnostics with phase/month and available identity. Fatal errors preserve the typed source error and do not promise recovery or persistent rollback semantics.
