# Property Simulation Sandbox (PSS)

**Project specification — 2026-10-21**
**Status:** M0–M4 milestone behavior has historical implementation/test coverage. A pre-M5 crate call-boundary review and remediation is complete; the former run, archive, and monthly coordinator implementations now exist only in test support. M5 is scoped to independent Core modules; M6 will integrate one monthly calculation. The application/data-access stage follows later. Ledger, mortgage, M3, and M4 vectors are indexed in `docs/test-vectors.md`.

The detailed specification is split by topic so each document has one main responsibility. Section numbers are retained from the original specification for reference; use the documents below as the current source of truth.

- [Project overview and product scope](docs/overview.md): goals, boundaries, concepts, input fixtures, and examples.
- [Domain model and monthly simulation](docs/domain-model.md): time, slices, events, randomness, ledger, monthly results, data contracts, and core domain rules.
- [Architecture and module boundaries](docs/architecture.md): ADR summary, Slice contract, Rust dependency direction, core types.
- [Architecture decision records](docs/adr/README.md): numbered decision archive and template.
- [Implementation roadmap and milestones](docs/implementation-roadmap.md): Core module work (M5), single-month integration (M6), later application/data-access and interface stages.
- [Engineering practices and quality gates](docs/engineering-practices.md): layering, lint/format, testing, review checklist.
- [Ledger and mortgage test vectors](docs/test-vectors.md): shared Ledger/mortgage calculation cases and historical M3/M4 regression vectors.
- [Desktop application design](docs/desktop-application.md): future GPUI workflows and UI principles.

## Current implementation order

1. Bottom modules: M0–M4 foundations, the completed pre-M5 crate boundary review, then M5 independent Core modules.
2. Middle calculation layer: M6 integrates initialization, independent module calls, monthly settlement, and audit for one month only.
3. Top application/data-access layer: a later stage loads user input/opening state/Ledger, passes values into M6, then persists user inputs, the complete monthly result (including its closing-state snapshot and Ledger-derived cash-flow sequence), and Ledger records. Ledger changes within the result and persisted Ledger records are the same economic facts, not a second ledger. Data access is called only by the application layer.
4. CLI/GPUI presentation: later interfaces constrain and parse user formats, present results, and invoke application use cases. The application layer alone coordinates the data access and monthly calculation flow. Multi-scenario and parallel scheduling are left for future high-level design.

For implementation decisions, the roadmap and domain model are authoritative. Update the relevant topic document when a decision changes; keep this index concise.
