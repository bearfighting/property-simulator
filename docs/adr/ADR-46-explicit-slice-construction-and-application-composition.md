# ADR-46: Explicit Slice construction and Application composition

- **Status:** Accepted; evaluation API amended by [ADR-54](ADR-54-slice-specific-evaluation-interfaces.md)
- **Date:** 2026-10-23
- **Related:** [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md), [ADR-41](ADR-41-application-available-slice-kinds.md), [Project structure](../project-structure.md)

## Context

The Slice crate had a shared factory trait that passed the full `Context` and `Snapshot` to every Slice and returned a vector of instances. This hid each Slice's actual inputs, duplicated a batch-creation interface for behaviors currently modeled as a single instance, and placed cross-Slice composition inside Core.

## Decision

- Remove the generic `SliceInstanceCreator::create_instances` interface and the Slice crate's `create_slices` dispatcher.
- Give each concrete Slice an explicit constructor whose typed parameters express the data needed to create that behavior. Do not pass the aggregate `Context` or `Snapshot` when a narrower input suffices.
- Application selects and constructs the configured Slices, invokes each independent calculation, and gathers their results. Slice-specific behavior remains in its owning Core module; Core does not add a global dispatcher or business-result aggregator.
- Application appends the returned journal entries to a candidate Ledger and publishes the monthly result only after the full month's work succeeds. Core Ledger and reporting APIs remain independently usable capabilities.
- Slice execution is coordinated by Application. The current scope does not implement parallel scheduling, and the Core contracts do not encode an execution order between independent Slices.
- Defer concrete constructor signatures until each Slice's inputs and lifecycle are defined. This decision does not rule out multiple instances where a domain behavior requires them.

## Alternatives considered

- Keep one generic factory receiving Context and Snapshot: rejected because it hides dependencies and gives every Slice access to unrelated state.
- Define a vector-returning creation method for every Slice: rejected as unnecessary for single-instance behaviors and premature for generated instances.
- Let each Slice read global settings directly: rejected because explicit inputs make dependencies and validation visible at the call site.

## Consequences

- Concrete Slice constructors and evaluations use explicit typed inputs. The choice to keep evaluation methods Slice-specific is recorded in ADR-54.
- Application owns enabled-Slice execution coordination without owning Slice domain calculations or requiring a fixed Core aggregation strategy.
- A generated behavior such as a deferred repair cost can still have an explicit constructor tied to its source event when that contract is implemented.
