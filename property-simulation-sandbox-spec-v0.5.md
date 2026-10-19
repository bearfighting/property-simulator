# Property Simulation Sandbox (PSS)

**Project specification v0.5 — 2026-10-19**
**Status:** Design baseline; implementation has not started.

The detailed specification is split by topic so each document has one main responsibility. Section numbers are retained from the original specification for reference; use the documents below as the current source of truth.

- [Project overview and product scope](docs/overview.md): goals, boundaries, concepts, scenario setup, examples.
- [Domain model and monthly simulation](docs/domain-model.md): time, slices, events, randomness, ledger, snapshots, data contracts, and core domain rules.
- [Architecture and module boundaries](docs/architecture.md): ADR summary, Slice contract, Rust dependency direction, core types.
- [Implementation roadmap and milestones](docs/implementation-roadmap.md): core-first phases, M0 checklist, M1–M5 acceptance criteria, baseline decisions.
- [Engineering practices and quality gates](docs/engineering-practices.md): layering, lint/format, testing, review checklist.
- [Desktop application design](docs/desktop-application.md): future GPUI workflows and UI principles.

## Current implementation order

1. Domain kernel: M0 engineering/design baseline, then M1 ledger primitives, M2 mortgage, M3 stochastic monthly runs and snapshot branches, M4 Slice composition, M5 stable core data contract.
2. Application and persistence: CLI, SQLite, input workflow, save/restore, exports.
3. Desktop application: GPUI scenario builder and simulation exploration.

For implementation decisions, the roadmap and domain model are authoritative. Update the relevant topic document when a decision changes; keep this index concise.
