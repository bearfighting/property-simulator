# Architecture decision records

Keep ADRs short and limited to decisions that affect current domain behavior or architecture. The roadmap and topic documents define the current implementation scope; update them with any accepted decision.

Use the next unused number; do not reuse retired numbers. An ADR records context, decision, alternatives and consequences. Do not keep implementation status reports or test logs in ADRs.

## Current decision

- [ADR-29](ADR-29-layered-calculation-and-application-boundaries.md): application/data-access boundary and agreed persistence requirements; exact SQLite representation remains open.
- [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md): focused value crates, cohesive Slice behavior, and open monthly-call interfaces.
- [ADR-34](ADR-34-simulation-random-root-lifecycle.md): create and persist one simulation random root in Context at initialization.
- [ADR-35](ADR-35-ordinal-month-periods.md): use zero-based ordinal months as simulation periods and per-simulation Snapshot keys.
- [ADR-36](ADR-36-ledger-capability-boundaries.md): historical direction for accounting capability boundaries; transaction/posting flow superseded by ADR-37.
- [ADR-37](ADR-37-slice-journal-entry-contract.md): Slice returns balanced JournalEntry values through a focused shared crate.
- [ADR-38](ADR-38-reporting-projections.md): reporting derives initial projections from read-only Ledger facts without calling Ledger.
- [ADR-39](ADR-39-context-settings-and-month-zero-start.md): Context stores initial/latest settings and the random root; simulations begin at Month 0.
- [ADR-40](ADR-40-separate-core-application-presentation-workspaces.md): Core, Application, and Presentation entrypoints use separate Cargo workspaces in one repository.
- [ADR-41](ADR-41-application-available-slice-kinds.md): supported Slice kinds include configured generated behavior; Slice grouping/order is deferred technical debt.
- [ADR-42](ADR-42-user-settings-and-opening-values.md): user settings contain opening values, stop conditions, and Slice settings; the random root remains a separate system-provided Context field.
- [ADR-43](ADR-43-required-simulation-name.md): Core Context requires a descriptive name; Application resolves omitted user input before construction.
- [ADR-44](ADR-44-exclude-frozen-crates-from-root-workspace.md): retain legacy `crates/` sources but remove them from the active root workspace.
- [ADR-45](ADR-45-use-root-workspace-for-core.md): list active Core crates in the repository-root Cargo workspace for tool discovery; keep frozen legacy crates excluded.

## Historical decisions

- [ADR-31](ADR-31-stateless-slice-runtime-and-repair-stages.md): prior centralized Slice invocation design; architecture superseded by ADR-33. Repair domain behavior remains recorded in the domain model.
- [ADR-32](ADR-32-repair-api-migration-sequence.md): completed historical sequence for removing the mixed repair API.

## New ADRs

Copy [TEMPLATE.md](TEMPLATE.md), assign the next number, and link it from the specification index when a consequential new decision is made.
