# ADR-44: Exclude frozen legacy crates from the root workspace

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** [ADR-40](ADR-40-separate-core-application-presentation-workspaces.md), [Project structure](../project-structure.md)
- **Verification:** root `cargo metadata` reports no legacy members; `core/Cargo.toml` continues to list the active Core workspace.

## Context

The repository-root workspace contains an earlier implementation that has been frozen as reference material. New domain design and implementation now happen in the separate `core/` workspace. Keeping the old crates as root-workspace members makes ordinary workspace commands include code that is no longer active.

## Decision

- Keep the old source and manifests under `crates/`; do not delete them.
- Set the root workspace member list to empty and explicitly exclude the old crate directories.
- Continue developing and checking Core through `core/Cargo.toml`. Future Application and Presentation workspaces remain separate.

## Alternatives considered

- Keep the legacy crates as root-workspace members: rejected because they are frozen and should not participate in active workspace checks.
- Delete the old crates: rejected because their domain behavior and implementation history remain useful references.
- Move old crates into Core: rejected because their existing boundaries and dependencies do not define the new Core architecture.

## Consequences

- Root workspace commands no longer build or test the frozen legacy crates.
- The old crate manifests still rely on root workspace inheritance and are not independently buildable while excluded; restoring them would require reattaching them to a workspace or giving them standalone manifests.
- The Core workspace remains independently discoverable and buildable from `core/Cargo.toml`.
