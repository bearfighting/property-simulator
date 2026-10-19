# ADR-11: Static Rust registration initially

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [architecture](../architecture.md)
- **Test vectors:** N/A; static composition is covered by M4 composition tests.

## Context
The initial model set is known and does not require runtime extensions.

## Decision
Use static Rust registration and concrete types initially. Do not add a dynamic plugin system, general configuration language, or async service architecture without a demonstrated need.

## Alternatives considered
- Dynamic plugins or a DSL: add abstraction and validation complexity without an initial use case.

## Consequences
New slices extend typed interfaces and coordinator composition; later changes remain possible at explicit boundaries.
