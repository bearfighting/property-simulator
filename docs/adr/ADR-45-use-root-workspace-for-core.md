# ADR-45: Use the repository-root workspace for Core

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [ADR-40](ADR-40-separate-core-application-presentation-workspaces.md), [ADR-44](ADR-44-exclude-frozen-crates-from-root-workspace.md), [Project structure](../project-structure.md)
- **Verification:** root `cargo metadata --no-deps` lists active Core packages with the repository root as `workspace_root`; `cargo check --workspace` succeeds.

## Context

Core is the active implementation and lives under `core/`. Keeping it in a nested workspace means tools opened at the repository root may not discover its crates reliably. Frozen legacy crates under `crates/` should remain available as reference material without being built as active members.

## Decision

- Define the active workspace in the repository-root `Cargo.toml` and list Core crates under `core/` as its members.
- Remove the nested `core/Cargo.toml` and `core/Cargo.lock`; the root manifest and lockfile own active Core workspace configuration.
- Keep frozen legacy crates under `crates/` and explicitly exclude them from the root workspace.
- Keep future Application and Presentation workspaces separate, as described in ADR-40.

## Alternatives considered

- Keep a nested Core workspace: rejected because Cargo tools launched from the repository root may not discover the active crates.
- Include the frozen legacy crates in the root workspace: rejected because they are no longer active and should not participate in normal workspace checks.
- Put future Application and Presentation crates in this workspace: rejected because those layers have separate ownership and dependency boundaries.

## Consequences

- Rust Analyzer and other Cargo-aware tools can discover active Core crates from the repository root.
- `cargo check --workspace` checks active Core crates; the root lockfile records their dependencies.
- The `core/` directory remains an organizational boundary, not a Cargo workspace boundary.
- The legacy sources remain on disk but excluded from ordinary workspace commands.
