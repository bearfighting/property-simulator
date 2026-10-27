# ADR-67: GPUI composition root and UI boundary

- **Status:** Accepted
- **Date:** 2026-10-25
- **Scope:** Initial GPUI desktop entrypoint and simulation initialization flow

## Context

The first user-facing workflow initializes a simulation. The GPUI view should
not own Application or persistence dependencies, while the program entrypoint
must initialize SQLite and coordinate the UI with the Application use case.

## Decision

- Keep GPUI presentation as a UI library and a desktop composition-root binary
  in the repository-root Cargo workspace.
- The UI library depends on GPUI Kit and exposes plain initialization form data
  plus an asynchronous host callback. It does not depend on Application, Core,
  or Data Access.
- The desktop binary constructs the SQLite pool and adapter, installs the
  schema, obtains the Application account catalogue, maps form values into the
  Application initialization request, and invokes the use case.
- Keep the account catalogue in Application configuration. The current chart
  includes posting roles for supported Slices and uses account classes for
  cash-flow classification; extend it when new posting roles or report
  categories are introduced.
- Run SQLx work on a Tokio runtime and return completion to the GPUI view
  asynchronously.

## Alternatives

- Let the view call Application directly: rejected because it makes UI
  components depend on use-case and domain types.
- Make GPUI itself the program composition root: rejected because startup must
  create persistence infrastructure and connect it to Application first.
- Start with a CLI entrypoint: rejected because the initial product interaction
  is a desktop initialization workflow.

## Consequences

- UI form values are a small presentation contract that the desktop entrypoint
  maps to Application types.
- The desktop binary is the integration point across Core, Data Access,
  Application, and the Presentation UI crate.
- The initial implementation stores its SQLite file at the process working
  directory; a user-facing data location can be selected with packaging.

## Verification examples

- `cargo check --manifest-path presentation/gpui/Cargo.toml` checks the UI and
  desktop integration.
- The UI crate manifest has no Application or Data Access dependencies.
- The desktop crate owns the SQLite pool and passes the adapter to the
  Application initialization use case.
