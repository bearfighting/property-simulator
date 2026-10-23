# ADR-44: Exclude frozen legacy crates from the root workspace

- **Status:** Superseded by [ADR-45](ADR-45-use-root-workspace-for-core.md)
- **Date:** 2026-10-23
- **Related:** [ADR-40](ADR-40-separate-core-application-presentation-workspaces.md), [Project structure](../project-structure.md)
- **Verification:** root `cargo metadata` reports no legacy members; active Core membership is now defined in the root workspace.

## Context

The repository-root workspace contains an earlier implementation that has been frozen as reference material. New domain design and implementation happen under `core/`. Keeping the old crates as active workspace members makes ordinary workspace commands include code that is no longer active.

## Decision

- Keep the old source and manifests under `crates/`; do not delete them.
- At the time of this decision, set the root workspace member list to empty and explicitly exclude the old crate directories.
- At the time of this decision, develop and check Core through `core/Cargo.toml`. Future Application and Presentation workspaces remain separate.

## Alternatives considered

- Keep the legacy crates as root-workspace members: rejected because they are frozen and should not participate in active workspace checks.
- Delete the old crates: rejected because their domain behavior and implementation history remain useful references.
- Move old crates into Core: rejected because their existing boundaries and dependencies do not define the new Core architecture.

## Consequences

- Root workspace commands no longer build or test the frozen legacy crates.
- The old crate manifests still rely on root workspace inheritance and are not independently buildable while excluded; restoring them would require reattaching them to a workspace or giving them standalone manifests.
- ADR-45 later moved active Core membership to the repository-root workspace; this ADR's decision to keep frozen legacy crates excluded remains in force.
