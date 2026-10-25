# ADR-40: Separate Core, Application, and Presentation workspaces

- **Status:** Superseded for Cargo workspace membership by [ADR-68](ADR-68-unified-root-cargo-workspace.md); layer dependency boundaries remain accepted
- **Date:** 2026-10-23
- **Scope:** Repository and Cargo workspace boundaries

## Context

The repository contains an existing implementation and a newer Core design area. Application use cases are introduced in their own workspace; user-facing entrypoints (CLI, GUI, or Web) have not yet been designed. Placing those layers inside Core would blur the dependency boundary and make Core depend on concerns that should remain above it.

## Decision

- Keep Core source crates under `core/`, separate from Application and Presentation workspaces. Cargo workspace membership is defined separately by ADR-45.
- Keep Application in its own Cargo workspace. The initial workspace now lives
  under `application/` and may depend on Core and Data Access.
- Keep each Presentation entrypoint (CLI, GUI, Web) in a separate Cargo workspace. Each entrypoint has a composition root that may depend on Application and Data Access; UI libraries expose presentation contracts and do not depend on Application directly. Application may depend on Core and Data Access.
- Core must not depend on Application, Presentation, or Data Access. This decision defines repository/workspace boundaries, not the crate APIs between layers.
- The repository root hosts Core membership without making Application or Presentation part of Core.

## Alternatives

- Put all new layers in one Cargo workspace: rejected because independent workspace boundaries make the dependency direction and ownership explicit and allow each layer to evolve separately.
- Put Application and Presentation crates under `core/`: rejected because those layers sit above Core and must not become Core members.
- Create every future workspace immediately: rejected because Presentation entrypoints have not been designed; create each workspace when that layer is designed.

## Consequences

- Core source remains grouped under `core/`; its workspace root is defined by ADR-45.
- Application has an independent workspace under `application/`; its initial
  use case initializes and persists a simulation without depending on SQLite.
- Application and Presentation can be built from sibling directories in one repository while keeping their manifests and dependency graphs separate.
- Cross-workspace build orchestration remains a separate concern; this ADR does not prescribe a monorepo build tool.

## Verification examples

- Core members are listed in the repository-root `Cargo.toml` and are checked with `cargo check --workspace`.
- The Application manifest may depend on Core and Data Access crates, while no Core manifest may depend on Application or Presentation crates.
- `cargo check --manifest-path application/Cargo.toml` checks the Application workspace independently of the Core root workspace.
- A Presentation UI manifest stays free of Application and Data Access dependencies; its composition-root binary owns those integrations.
