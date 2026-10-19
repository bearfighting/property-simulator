# ADR-20: Composable slices and decoupled random sampling

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [ADR-05](ADR-05-slice-coordination.md), [ADR-06](ADR-06-events-effects-ledger.md), [ADR-09](ADR-09-random-trials.md), [ADR-12](ADR-12-independent-random-categories.md), [ADR-17](ADR-17-money-and-identifiers.md), [ADR-18](ADR-18-sufficient-state-transition.md), [ADR-19](ADR-19-validation-only-domain-errors.md)
- **Test vectors:** Independent proposal merging must be invariant under permutation and partitioning; sampled values are supplied explicitly to Slice calculations.

## Context

M2 needs to support independent activity calculations, parallel or grouped evaluation, traceable outputs, and experiments with different random sampling methods. Coupling a Slice to a particular sampler would make those experiments invasive. Runtime failures also need a clear boundary between expected input validation and unrecoverable implementation or infrastructure faults.

## Decision

Independent Slices evaluate against the same immutable opening state and frozen period inputs. Their proposals are combined with an operation that is associative and has an identity. For proposals that are independent and have no conflicting writes, combination must also be commutative. Therefore regrouping or reordering an independent subset cannot change the combined result. A Slice's state transformation is not required to commute with another transformation; shared writes must instead be excluded by validation or handled by an explicit deterministic rule/stage.

Activities are configured as Slice instances with caller-assigned stable instance IDs. A Slice supplies output keys and proposal content; successful Slice evaluation returns an opaque output bound to the exact activity instance and frozen period that produced it. ID assignment accepts this bound output, so callers cannot relabel proposal provenance between evaluation and identification. The coordinator attaches the authoritative instance, Slice, model version, and period provenance, then derives stable output IDs from the trial/branch, period, instance ID, output kind, and output key. IDs must not depend on execution order, batching, or worker scheduling. Re-evaluating the same instance in the same trial and period produces the same output IDs. Combined output collections use canonical stable-ID ordering.

The injected ID factory must be deterministic and collision-resistant. Since IDs have a fixed 128-bit representation and output keys are not length-bounded, mathematical injectivity over all possible inputs is not promised. The monthly coordination boundary rejects duplicate IDs within each output kind, and a period result retains its trial and branch identity so outputs from different runs cannot be combined accidentally.

Probability distributions, sampling algorithms, and Slices are separate components. A distribution defines the supported values and their probability law; a sampler implements a method for producing a value from that distribution; a Slice consumes typed, explicit sampled values and does not call or own a random-number generator. Sample provenance (instance, Slice, model version, sample kind, and period) is checked against the receiving Slice and frozen period, and the period result checks the sample kind against the activity instance. Different compatible samplers may be paired with the same distribution, and the same Slice may consume values produced by different samplers. Compatibility is validated at composition boundaries. The distribution's full support must fit validated numeric and domain constraints; unbounded support requires explicit truncation/value semantics or rejection of that configuration. M2 may inject explicit samples; random-stream allocation remains in M3.

Unexpected runtime errors, including checked numeric overflow outside validated supported bounds, are fatal simulation failures and are not user-correctable domain outcomes. The run stops and produces diagnostics for engineering investigation. The system does not promise recovery, rollback, or persistent-state consistency after such an unrecoverable failure. Successful period completion still requires the normal state, event, and ledger result to satisfy their invariants.

## Alternatives considered

- Require every Slice's state transformation to commute: rejected because explicit dependencies and ordered stages may be necessary; the algebraic guarantee applies to combining independent proposals.
- Let a Slice call a sampler directly: rejected because it couples domain rules to sampling algorithms and prevents clean sampler experiments.
- Treat fatal runtime failures as recoverable user input errors: rejected because it misattributes implementation or infrastructure failures.
- Guarantee restartable, transactionally consistent persistence after every fatal runtime failure: deferred; recovery and persistence semantics belong to later storage/run-control work and are not promised for unrecoverable failures.

## Consequences

M2 defines explicit proposal merge rules and stable provenance/ID derivation. It tests independent results under reordered and partitioned evaluation, while testing conflict rules separately. Slice tests use explicit typed samples; distribution and sampler tests are independent, with compatibility tests at their composition boundary. Runtime failure handling stops the run and records diagnostics; no simulation result is emitted for a failed period.
