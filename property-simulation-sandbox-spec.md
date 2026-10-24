# Property Simulation Sandbox

Project specification index. Use the following current documents as the source of truth:

- [Project scope](docs/overview.md): product purpose and explicit exclusions.
- [Domain model](docs/domain-model.md): state, monthly calculation, accounting, randomness and persistence invariants.
- [Architecture](docs/architecture.md): runtime responsibilities and dependency direction.
- [UI input defaults](docs/ui-ux-design.md): user-owned settings, default scenario and enabled Slice behavior.
- [Current work plan](docs/implementation-roadmap.md): Core boundary clarification followed by Application/Data Access work.
- [Engineering practices](docs/engineering-practices.md): code boundaries, tests and quality commands.
- [Ledger and mortgage vectors](docs/test-vectors.md): compact numerical examples defining current accounting and loan behavior.
- [Architecture decision](docs/adr/ADR-29-layered-calculation-and-application-boundaries.md): concise rationale for the application/data-access boundary.
- [Core boundary direction](docs/adr/ADR-33-core-boundaries-and-slice-cohesion.md): separate primitive value crates and keep domain behavior cohesive with its Slice.
- [Simulation random root lifecycle](docs/adr/ADR-34-simulation-random-root-lifecycle.md): generate and persist a simulation random root in Context at initialization.
- [Ordinal month periods](docs/adr/ADR-35-ordinal-month-periods.md): use zero-based simulation months as period values and Snapshot keys.
- [Ledger capability boundaries](docs/adr/ADR-36-ledger-capability-boundaries.md): historical accounting boundary direction; transaction/posting flow superseded by ADR-37.
- [Slice journal-entry contract](docs/adr/ADR-37-slice-journal-entry-contract.md): Slice returns balanced, month-scoped journal entries through a focused shared value crate.
- [Reporting projections](docs/adr/ADR-38-reporting-projections.md): reporting consumes read-only Ledger facts and derives reports without calling Ledger.
- [Context settings and Month 0 start](docs/adr/ADR-39-context-settings-and-month-zero-start.md): Context stores initial/latest settings and root key; each simulation begins at Month 0.
- [Workspace boundaries](docs/adr/ADR-40-separate-core-application-presentation-workspaces.md): Core, Application, and CLI/GUI/Web entrypoints use separate Cargo workspaces.
- [Slice catalogue and deferred grouping](docs/adr/ADR-41-application-available-slice-kinds.md): supported `SliceKind` values, generated repair-cost behavior, and deferred dependency grouping.
- [User settings and opening values](docs/adr/ADR-42-user-settings-and-opening-values.md): opening inputs, stop-condition timing, and the separate system-provided random root.
- [Required simulation name](docs/adr/ADR-43-required-simulation-name.md): Core requires a descriptive simulation name; Application supplies a fallback when the user omits one.
- [Frozen legacy crates](docs/adr/ADR-44-exclude-frozen-crates-from-root-workspace.md): legacy sources remain in the repository but are excluded from the active root workspace.
- [Core workspace discovery](docs/adr/ADR-45-use-root-workspace-for-core.md): active Core crates are members of the repository-root workspace; Application and Presentation remain separate.
- [Explicit Slice construction](docs/adr/ADR-46-explicit-slice-construction-and-application-composition.md): Slice construction receives explicit typed inputs; Application coordinates enabled Slice composition.
- [Pure loan calculations](docs/adr/ADR-47-pure-loan-calculations.md): pure amortization arithmetic lives in `domain-calculations`; Loan Slice validates and assembles domain/accounting outputs.
- [Monthly Snapshot contents](docs/adr/ADR-48-monthly-snapshot-content-and-continuation.md): Snapshot stores effective inputs and completed-month outputs, with continuation state distinct from Ledger-derived balances.
- [Slice contract catalogue](docs/adr/ADR-49-slice-catalogue-owns-slice-contracts.md): `slice-catalogue` defines Slice kinds, settings, per-period inputs, and typed outputs shared by Context, Slice, and Snapshot.
- [Required account roles](docs/adr/ADR-52-account-role-is-required-and-unique.md): each account has a unique semantic role in its catalogue; JournalEntry postings use the corresponding AccountCode.
- [Recurring fixed expense Slice](docs/adr/ADR-53-recurring-fixed-expense-slice.md): combine property-related taxes and utility costs into one recurring Slice, configured by monthly amount or annual property-value rate.
- [Slice-specific evaluation interfaces](docs/adr/ADR-54-slice-specific-evaluation-interfaces.md): inject random input only into stochastic Slices and keep evaluation methods/errors concrete.
- [ADR guidance](docs/adr/README.md): how to record future consequential decisions.

Code identifiers must not contain implementation-stage names or numbers. The architecture document records the current boundary direction and open interface questions. The roadmap starts with resolving those Core interfaces before Application/Data Access implementation. Historical decisions are retained only as decision history.
