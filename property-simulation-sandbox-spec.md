# Property Simulation Sandbox

Project specification index. Use the following current documents as the source of truth:

- [Project scope](docs/overview.md): product purpose and explicit exclusions.
- [Domain model](docs/domain-model.md): state, monthly calculation, accounting, randomness and persistence invariants.
- [Architecture](docs/architecture.md): runtime responsibilities and dependency direction.
- [UI input defaults](docs/ui-ux-design.md): user-owned settings, default scenario and enabled Slice behavior.
- [Current work plan](docs/implementation-roadmap.md): Core is the current implementation baseline; work proceeds on Application and the Data Access contract.
- [Engineering practices](docs/engineering-practices.md): code boundaries, tests and quality commands.
- [Ledger and mortgage vectors](docs/test-vectors.md): compact numerical examples defining current accounting and loan behavior.
- [Architecture decision](docs/adr/ADR-29-layered-calculation-and-application-boundaries.md): concise rationale for the application/data-access boundary.
- [Core boundary direction](docs/adr/ADR-33-core-boundaries-and-slice-cohesion.md): separate primitive value crates and keep domain behavior cohesive with its Slice.
- [Simulation random root lifecycle](docs/adr/ADR-34-simulation-random-root-lifecycle.md): historical Context ownership superseded by ADR-59.
- [Ordinal month periods](docs/adr/ADR-35-ordinal-month-periods.md): Month 0 is initialization; Month 1 is the first calculated period.
- [Ledger capability boundaries](docs/adr/ADR-36-ledger-capability-boundaries.md): historical accounting boundary direction; transaction/posting flow superseded by ADR-37.
- [Slice journal-entry contract](docs/adr/ADR-37-slice-journal-entry-contract.md): Slice returns balanced, month-scoped journal entries through a focused shared value crate.
- [Reporting projections](docs/adr/ADR-38-reporting-projections.md): reporting consumes read-only Ledger facts and derives reports without calling Ledger.
- [Context settings and Month 0 initialization](docs/adr/ADR-39-context-settings-and-month-zero-start.md): historical Context design; each simulation initializes at Month 0.
- [Workspace boundaries](docs/adr/ADR-40-separate-core-application-presentation-workspaces.md): Core, Application, and CLI/GUI/Web entrypoints use separate Cargo workspaces.
- [Slice catalogue and deferred grouping](docs/adr/ADR-41-application-available-slice-kinds.md): closed supported `SliceKind` values and generated repair-cost behavior.
- [User settings and opening values](docs/adr/ADR-42-user-settings-and-opening-values.md): historical settings aggregate direction; random-root ownership is superseded by ADR-59.
- [Required simulation name](docs/adr/ADR-43-required-simulation-name.md): historical Core name value; simulation names are Application metadata.
- [Frozen legacy crates](docs/adr/ADR-44-exclude-frozen-crates-from-root-workspace.md): legacy sources remain in the repository but are excluded from the active root workspace.
- [Core workspace discovery](docs/adr/ADR-45-use-root-workspace-for-core.md): active Core crates are members of the repository-root workspace; Application and Presentation remain separate.
- [Explicit Slice construction](docs/adr/ADR-46-explicit-slice-construction-and-application-composition.md): Slice construction receives explicit typed inputs; Application coordinates enabled Slice composition.
- [Pure loan calculations](docs/adr/ADR-47-pure-loan-calculations.md): pure amortization arithmetic lives in `domain-calculations`; Loan Slice validates and assembles domain/accounting outputs.
- [Monthly Snapshot contents](docs/adr/ADR-48-monthly-snapshot-content-and-continuation.md): historical Snapshot design superseded by ADR-60.
- [Slice contract catalogue](docs/adr/ADR-49-slice-catalogue-owns-slice-contracts.md): shared per-Slice contracts remain; aggregate ownership is superseded by ADR-60.
- [Required account roles](docs/adr/ADR-52-account-role-is-required-and-unique.md): each account has a unique semantic role in its catalogue; JournalEntry postings use the corresponding AccountCode.
- [Recurring fixed expense Slice](docs/adr/ADR-53-recurring-fixed-expense-slice.md): combine property-related taxes and utility costs into one recurring Slice, configured by monthly amount or annual property-value rate.
- [Slice-specific evaluation interfaces](docs/adr/ADR-54-slice-specific-evaluation-interfaces.md): inject random input only into stochastic Slices and keep evaluation methods/errors concrete.
- [Ledger journal as the source of balances](docs/adr/ADR-55-ledger-journal-as-source-of-balances.md): represent opening facts in a Month 0 JournalEntry, inject the account catalogue, and derive balances from journal entries.
- [ADR guidance](docs/adr/README.md): how to record future consequential decisions.
- [Stop conditions and month boundaries](docs/adr/ADR-56-stop-conditions-and-month-boundaries.md): represent manual stop as a typed condition and apply conditions at the next month boundary.
- [Per-Slice settings persistence](docs/adr/ADR-57-per-slice-settings-persistence.md): persist typed configuration per static Slice kind and keep initialization facts in Month 0 Ledger entries.
- [Stop condition domain boundary](docs/adr/ADR-58-stop-condition-domain-boundary.md): keep stop condition values and pure evaluation independent of Context, Ledger, and Slice.
- [Application-owned random root](docs/adr/ADR-59-application-owned-random-root.md): keep the simulation root in Application-owned state and persist it separately from settings and monthly results.
- [Core simulation contracts without aggregates](docs/adr/ADR-60-core-simulation-contracts-without-aggregates.md): keep per-Slice contracts in Core and configuration/month aggregation in Application.
- [Derive Slice continuation from domain facts](docs/adr/ADR-64-derive-slice-continuation-from-domain-facts.md): omit generic Effect records while current continuation inputs are derivable from Month, Events, configuration, and Ledger.
- [Persist pending repair costs](docs/adr/ADR-65-persist-pending-repair-cost-instances.md): persist generated repair cost instances and prevent retroactive edits to settings used by completed months.
- [Application and Data Access static-dispatch boundary](docs/adr/ADR-66-application-data-access-static-dispatch.md): define storage-neutral semantic persistence traits and inject SQLite through generic static dispatch.
- [GPUI composition root and UI boundary](docs/adr/ADR-67-gpui-composition-root-and-ui-boundary.md): keep GPUI views dependent on plain form contracts and let the desktop entrypoint coordinate Application and persistence.
- [Unified root Cargo workspace](docs/adr/ADR-68-unified-root-cargo-workspace.md): register active crates in one workspace and make the desktop binary the default root command.
- [GPUI page shell and reusable components](docs/adr/ADR-69-gpui-page-shell-and-reusable-components.md): organize page navigation and UI composition for initialization, monthly simulation, and summary views.
- [Immutable simulation configuration](docs/adr/ADR-70-immutable-simulation-configuration.md): configuration is fixed for a simulation's lifetime; use a separate simulation for different assumptions.
- [In-memory simulation lifecycle coordinator](docs/adr/ADR-71-in-memory-simulation-lifecycle-coordinator.md): an independent lifecycle crate constrains which Application work unit the composition layer may call.

Code identifiers must not contain implementation-stage names or numbers. The architecture document records the current boundary direction and open interface questions. The roadmap tracks Application/Data Access work; Core may evolve incrementally as concrete needs arise. Historical decisions are retained only as decision history.
