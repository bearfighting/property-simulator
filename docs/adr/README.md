# Architecture decision records

Keep ADRs short and limited to decisions that affect current domain behavior or architecture. The roadmap and topic documents define the current implementation scope; update them with any accepted decision.

Use the next unused number; do not reuse retired numbers. An ADR records context, decision, alternatives and consequences. Do not keep implementation status reports or test logs in ADRs.

## Current decision

- [ADR-29](ADR-29-layered-calculation-and-application-boundaries.md): application/data-access boundary and agreed persistence requirements; exact SQLite representation remains open.
- [ADR-33](ADR-33-core-boundaries-and-slice-cohesion.md): focused value crates, cohesive Slice behavior, and open monthly-call interfaces.
- [ADR-34](ADR-34-simulation-random-root-lifecycle.md): historical random-root lifecycle decision; Context ownership superseded by ADR-59.
- [ADR-35](ADR-35-ordinal-month-periods.md): use Month 0 as initialization and Month 1 as the first calculated period.
- [ADR-36](ADR-36-ledger-capability-boundaries.md): historical direction for accounting capability boundaries; transaction/posting flow superseded by ADR-37.
- [ADR-37](ADR-37-slice-journal-entry-contract.md): Slice returns balanced JournalEntry values through a focused shared crate.
- [ADR-38](ADR-38-reporting-projections.md): reporting derives initial projections from read-only Ledger facts without calling Ledger.
- [ADR-39](ADR-39-context-settings-and-month-zero-start.md): historical Context design; Core aggregate ownership superseded by ADR-60, root placement by ADR-59.
- [ADR-40](ADR-40-separate-core-application-presentation-workspaces.md): Core, Application, and Presentation entrypoints use separate Cargo workspaces in one repository.
- [ADR-41](ADR-41-application-available-slice-kinds.md): use a closed static `SliceKind` set; generated repair behavior remains supported.
- [ADR-42](ADR-42-user-settings-and-opening-values.md): historical UserSettings design; root placement superseded by ADR-59.
- [ADR-43](ADR-43-required-simulation-name.md): historical Core name value; simulation names are Application metadata.
- [ADR-44](ADR-44-exclude-frozen-crates-from-root-workspace.md): retain legacy `crates/` sources but remove them from the active root workspace.
- [ADR-45](ADR-45-use-root-workspace-for-core.md): list active Core crates in the repository-root Cargo workspace for tool discovery; keep frozen legacy crates excluded.
- [ADR-46](ADR-46-explicit-slice-construction-and-application-composition.md): construct Slice behavior from explicit typed inputs and leave cross-Slice assembly to Application.
- [ADR-47](ADR-47-pure-loan-calculations.md): keep fixed-rate loan arithmetic in pure domain calculations and Slice-specific validation/output mapping in Loan Slice.
- [ADR-48](ADR-48-monthly-snapshot-content-and-continuation.md): historical Snapshot design superseded by ADR-60.
- [ADR-49](ADR-49-slice-catalogue-owns-slice-contracts.md): shared per-Slice contracts remain; Context/Snapshot ownership is superseded by ADR-60.
- [ADR-50](ADR-50-context-is-source-of-current-settings.md): historical Context settings source superseded by ADR-60.
- [ADR-51](ADR-51-slice-owned-continuation-state.md): keep Slice-specific continuation changes in typed Slice results, not as Snapshot fields or journal notes.
- [ADR-52](ADR-52-account-role-is-required-and-unique.md): require one stable account role for each account code and resolve posting codes through the account catalogue.
- [ADR-53](ADR-53-recurring-fixed-expense-slice.md): combine recurring property-related taxes and utility costs into one fixed-expense Slice configured by monthly amount or annual property-value rate.
- [ADR-54](ADR-54-slice-specific-evaluation-interfaces.md): inject random input only into stochastic Slices and keep evaluation methods/errors concrete.
- [ADR-55](ADR-55-ledger-journal-as-source-of-balances.md): make the ordered JournalEntry sequence the sole source for opening and ending Ledger balances; inject the account catalogue when needed.
- [ADR-56](ADR-56-stop-conditions-and-month-boundaries.md): represent manual stop as `UserStop` and evaluate conditions before the next month.
- [ADR-57](ADR-57-per-slice-settings-persistence.md): persist typed configuration per static Slice kind, keep loan terms with Loan, and avoid a whole-settings blob.
- [ADR-58](ADR-58-stop-condition-domain-boundary.md): define stop conditions and pure boundary evaluation without depending on Context, Ledger, or Slice.
- [ADR-59](ADR-59-application-owned-random-root.md): keep the simulation root in Application-owned state and persist it independently of settings and monthly results.
- [ADR-60](ADR-60-core-simulation-contracts-without-aggregates.md): keep per-Slice contracts in Core, while Application owns configuration collections and monthly aggregation.
- [ADR-61](ADR-61-asynchronous-sqlite-data-access.md): keep SQLite mapping in an async adapter, inject the host-created pool, and leave DTOs storage-neutral.
- [ADR-62](ADR-62-month-scoped-event-facts.md): persist month-scoped event kinds without monetary payloads or JournalEntry links; derive amounts from Ledger.
- [ADR-63](ADR-63-single-loan-initial-scope.md): support one loan per simulation initially; multi-loan support requires a broader settings and continuation redesign.
- [ADR-64](ADR-64-derive-slice-continuation-from-domain-facts.md): remove generic Slice Effects while current continuation inputs are derivable from Month, Events, configuration, and Ledger.
- [ADR-65](ADR-65-persist-pending-repair-cost-instances.md): persist generated repair costs until settlement and protect completed-month settings history.
- [ADR-66](ADR-66-application-data-access-static-dispatch.md): define semantic Data Access traits separately from SQLite and inject adapters through generic static dispatch.

## Historical decisions

- [ADR-31](ADR-31-stateless-slice-runtime-and-repair-stages.md): prior centralized Slice invocation design; architecture superseded by ADR-33. Repair domain behavior remains recorded in the domain model.
- [ADR-32](ADR-32-repair-api-migration-sequence.md): completed historical sequence for removing the mixed repair API.

## New ADRs

Copy [TEMPLATE.md](TEMPLATE.md), assign the next number, and link it from the specification index when a consequential new decision is made.
