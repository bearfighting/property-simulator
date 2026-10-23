# ADR-40: Separate Core, Application, and Presentation workspaces

- **Status:** Accepted direction
- **Date:** 2026-10-23
- **Scope:** Repository and Cargo workspace boundaries

## Context

The repository contains an existing implementation and a newer Core design area. Application orchestration and user-facing entrypoints (CLI, GUI, or Web) have not yet been designed. Placing those layers inside Core would blur the dependency boundary and make Core depend on concerns that should remain above it.

## Decision

- Keep Core source crates under `core/`, separate from Application and Presentation workspaces. Cargo workspace membership is defined separately by ADR-45.
- Keep Application in its own Cargo workspace when that layer is introduced.
- Keep each Presentation entrypoint (CLI, GUI, Web) in a separate Cargo workspace when introduced. These workspaces may live in this repository and depend on Application; Application may depend on Core.
- Core must not depend on Application, Presentation, or Data Access. This decision defines repository/workspace boundaries, not the crate APIs between layers.
- Keep Application in its own Cargo workspace when that layer is introduced. The repository root can host Core membership without making Application or Presentation part of Core.

## Alternatives

- Put all new layers in one Cargo workspace: rejected because independent workspace boundaries make the dependency direction and ownership explicit and allow each layer to evolve separately.
- Put Application and Presentation crates under `core/`: rejected because those layers sit above Core and must not become Core members.
- Create every future workspace immediately: rejected because no Application or Presentation implementation exists yet; create each workspace when that layer is designed.

## Consequences

- Core source remains grouped under `core/`; its workspace root is defined by ADR-45.
- Future layers can be built from sibling directories in one repository while keeping their manifests and dependency graphs separate.
- Cross-workspace integration and shared build orchestration remain future implementation concerns; this ADR does not prescribe a monorepo build tool or workspace membership across directories.

## Verification examples

- Core members are listed in the repository-root `Cargo.toml` and are checked with `cargo check --workspace`.
- A future Application manifest may depend on Core crates, while no Core manifest may depend on Application or Presentation crates.
