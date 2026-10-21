# Repository guidance

## Sources of truth

- Read `property-simulation-sandbox-spec.md`, then consult the relevant topic document before changing domain behavior or architecture.
- `docs/implementation-roadmap.md` defines the current application/data-access work; `docs/domain-model.md` defines simulation and accounting semantics; `docs/architecture.md` defines dependency boundaries; `docs/engineering-practices.md` defines code-quality gates.
- Record consequential new domain or architecture decisions in short ADRs under `docs/adr/`, including alternatives, consequences and verification examples where useful.

## Domain rules

- Keep Core independent of UI, CLI, SQLite, files and other IO. UI calls Application; only Application calls Data Access.
- Treat time as inclusive `YearMonth` intervals. Support any valid, sufficiently complete opening state and do not impose a fixed simulation duration.
- Preserve user-provided and generated state required for continuation. Distinguish initial facts from events generated during the simulated interval; do not invent past transactions.
- The calculation is synchronous. Do not model household solvency, bankruptcy, delinquency/default or foreclosure in the initial scope. Negative property cash flow and other computable unusual outcomes are results, not failures.
- Keep events, decisions, effects and Ledger transactions distinct and traceable.
- Slice calculations use explicit inputs and injected interfaces; they do not mutate shared state or Ledger. Single-month calculation composes Core modules and settles a candidate Ledger without IO or cross-month state.
- Use decimal fixed-point money, explicit currency and documented rounding. Posted positive amounts round half-up to the currency minor unit; do not use binary floating point for posted amounts.
- Use double-entry accounting. Every posted transaction balances. Derive balances and cash-flow/financial statement views from opening balances and posted entries; do not maintain competing hand-written totals.
- Keep the initial accounting profile focused on household property assets, financing, rental income and operating expenses.
- Follow the mortgage rate conversion, accrual/payment order, precision and rounding in `docs/domain-model.md`.
- Keep distributions, samplers and random-source allocation separate. Core consumes explicit random input and does not access system entropy or own random streams. Application obtains randomness from the system; repeatability and true randomness are not product guarantees. Random state may be persisted with context or snapshot for later months; exact placement is an open data-access design choice.
- Treat random categories as independent initially. Do not add hidden cross-category correlation.

## Application and persistence

- Application is the only coordinator between Presentation, Data Access and single-month calculation. Do not add stage names or numbers to production crate, module, type, function or field names.
- The simulation context retains initial user settings, latest settings and commands. Persist the effective inputs used by every completed month.
- Users may configure multiple stop conditions; any satisfied condition stops the simulation. A stop command in a monthly update does not calculate another month.
- Persist context edits in the context-update use case and save each completed month before advancing. A storage failure stops the simulation and is reported as a system error; do not add compensation or recovery behavior.
- Persist the opening Ledger facts and committed transactions. Preserve Ledger-derived cash-flow details by activity instance/Slice and transaction; a cash-flow total alone is insufficient.
- A fatal calculation or storage failure stops the simulation. Do not implement failed-run recovery, retry, replay, branches, multi-scenario management or parallel scheduling.
- Do not promise schema versioning, migrations or cross-application-upgrade compatibility. The application version is the only software version identifier. Long-term retention after simulation completion is outside current work.

## Implementation and verification

- Prefer small typed interfaces and explicit validation over a general DSL, plugin framework or speculative abstraction. Explain responsibility and dependency direction for new modules.
- Use the repository's `./scripts/check.sh` workflow for formatting, Clippy, compilation and relevant tests. Keep warnings visible and justify narrow lint exceptions. Do not claim checks that were not run.
- Test domain rules without IO. Test Application with fake/in-memory Data Access and test SQLite at its serialization/storage boundary. Use fixed inputs and seeds where deterministic tests need them.
- Do not add CLI/GPUI dependencies to Core crates. Report what changed and which checks were run before finishing.
