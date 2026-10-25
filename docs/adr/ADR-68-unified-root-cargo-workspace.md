# ADR-68: Use one root Cargo workspace for active crates

- **Status:** Accepted
- **Date:** 2026-10-25
- **Related:** [ADR-40](ADR-40-separate-core-application-presentation-workspaces.md), [ADR-45](ADR-45-use-root-workspace-for-core.md), [GPUI composition root](ADR-67-gpui-composition-root-and-ui-boundary.md)
- **Scope:** Cargo workspace membership and the default desktop command

## Context

Core, Data Access, Application, and GPUI had separate Cargo workspaces. That
kept build commands isolated, but the program could not be launched from the
repository root with a plain `cargo run`. The Rust crate dependency boundaries
already express the layer separation independently of workspace membership.

## Decision

- Register all active Core, Data Access, Application, and GPUI crates as
  members of the repository-root Cargo workspace.
- Keep frozen legacy crates under `crates/` excluded from the active workspace.
- Set `presentation/gpui/desktop` as the sole default member so `cargo run`
  from the repository root selects the desktop binary.
- Keep dependency direction explicit in crate manifests: GPUI UI depends on
  GPUI Kit; the desktop composition root connects UI, Application, and SQLite.
- Use the root `Cargo.lock` for the active workspace.

## Alternatives

- Keep separate workspaces and invoke `cargo run` with a manifest path and
  package selector: rejected because the normal project launch should be
  available from the repository root.
- Make every crate a default member: rejected because Cargo would have no
  single desktop target to run by default.

## Consequences

- Root workspace checks cover all active crates and may take longer because
  the GPUI desktop dependencies are included.
- `cargo run` launches the GPUI desktop program; `cargo check --workspace`
  checks all active layers.
- Workspace membership does not make Core depend on upper layers. Dependency
  boundaries continue to be enforced by crate manifests and code review.

## Verification examples

- `cargo metadata --no-deps --format-version 1` lists all active crates with
  the repository root as their workspace root and desktop as the sole default
  member.
- `cargo run` from the repository root selects `property-simulation-desktop`.
- `cargo check --workspace --all-targets` checks every active package.
