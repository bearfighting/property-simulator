# Repository guidance

## Sources of truth

- Read `property-simulation-sandbox-spec.md`, then consult the relevant topic document before changing domain behavior or architecture.
- `docs/implementation-roadmap.md` defines the current application/data-access work; `docs/domain-model.md` defines simulation and accounting semantics; `docs/architecture.md` defines dependency boundaries; `docs/engineering-practices.md` defines code-quality gates.
- Record consequential new domain or architecture decisions in short ADRs under `docs/adr/`, including alternatives, consequences and verification examples where useful.

## Domain rules

- Keep Core independent of UI, CLI, SQLite, files and other IO. UI calls Application; only Application calls Data Access.
- Treat simulation periods as inclusive `Month` intervals. `Month` is a zero-based ordinal within one simulation, not a calendar date or globally unique ID. Every simulation begins at `Month(0)`; opening Ledger facts precede that first period. Continue from the latest completed Snapshot, and do not impose a fixed simulation duration.
- Preserve user-provided and generated state required for continuation. Distinguish initial facts from events generated during the simulated interval; do not invent past transactions.
- The calculation is synchronous. Do not model household solvency, bankruptcy, delinquency/default or foreclosure in the initial scope. Negative property cash flow and other computable unusual outcomes are results, not failures.
- Keep events, decisions, effects and Ledger transactions distinct and traceable.
- Slice calculations use explicit inputs and injected interfaces; they do not mutate shared state or Ledger. Single-month calculation composes Core modules and settles a candidate Ledger without IO or cross-month state.
- Use decimal fixed-point money, explicit currency and documented rounding. Posted positive amounts round half-up to the currency minor unit; do not use binary floating point for posted amounts.
- Use double-entry accounting. Every posted transaction balances. Derive balances and cash-flow/financial statement views from opening balances and posted entries; do not maintain competing hand-written totals.
- Keep the initial accounting profile focused on household property assets, financing, rental income and operating expenses.
- Follow the mortgage rate conversion, accrual/payment order, precision and rounding in `docs/domain-model.md`.
- Keep distributions, samplers and random-source allocation separate. Core consumes explicit random input and does not access system entropy or own random streams. Application obtains a simulation root from the system and saves it in Context at initialization for use until the simulation ends. Random engine derives inputs from the root and explicit semantic identity; Snapshot need not duplicate stream state when using this derivation. Repeatability and true randomness are not product guarantees.
- Treat random categories as independent initially. Do not add hidden cross-category correlation.

## Application and persistence

- Application is the only coordinator between Presentation, Data Access and single-month calculation. Do not add stage names or numbers to production crate, module, type, function or field names.
- Context retains initial user settings, optional latest user settings, and the simulation random root. Initial settings remain unchanged; accepted updates set latest settings. When no latest settings exist, initial settings are effective. Do not retain a separate command history in Context. Persist the effective inputs used by every completed month.
- Users may configure multiple stop conditions; any satisfied condition stops the simulation. A stop command in a monthly update does not calculate another month.
- Persist context edits in the context-update use case and save each completed month before advancing. A storage failure stops the simulation and is reported as a system error; do not add compensation or recovery behavior.
- Persist the opening Ledger facts and committed transactions. Preserve Ledger-derived cash-flow details by activity instance/Slice and transaction; a cash-flow total alone is insufficient.
- A fatal calculation or storage failure stops the simulation. Do not implement failed-run recovery, retry, replay, branches, multi-scenario management or parallel scheduling.
- Do not promise schema versioning, migrations or cross-application-upgrade compatibility. The application version is the only software version identifier. Long-term retention after simulation completion is outside current work.

## Implementation and verification

- Prefer small typed interfaces and explicit validation over a general DSL, plugin framework or speculative abstraction. Explain responsibility and dependency direction for new modules.
- Use kebab-case (hyphens) for Cargo package names and their directory names (for example, `slice-catalogue`). Use snake_case (underscores) for Rust source filenames and module identifiers (for example, `slice_catalogue.rs` / `slice_catalogue`). Rust crate paths use the package name with underscores (`slice_catalogue`).
- For Rust modules, use the file named after the module (for example, `slices.rs` with children under `slices/`) instead of `mod.rs`.
- Keep crate roots (`lib.rs`) and module roots (files that declare child modules) limited to imports, module declarations, and the public or crate-visible interfaces/re-exports needed by their callers. Put implementation in the file named after the module or in its child modules.
- Group methods in `impl` blocks by responsibility; use separate blocks for logically distinct capabilities, without splitting related methods mechanically.
- Mark intentionally incomplete public `struct`, `trait`, and `fn` items in their doc comments. State what is incomplete, what consumers must not assume yet, and concrete conditions for completion (including needed domain decisions or examples). Replace or remove the marker when those conditions are met; avoid vague TODO-only comments.
- Use blank lines to make code structure readable: separate adjacent `struct`/`enum` and `fn` definitions, and separate logically distinct statement groups inside functions. Keep tightly related statements together; do not add blank lines mechanically between every statement.
- Treat files above roughly 300 lines as a prompt to review cohesion and readability. When a file mixes implementation and tests, move tests into a separate test module/file if that makes the production code easier to read; do not split mechanically when it would obscure the code's structure or require awkward visibility.
- Use the repository's `./scripts/check.sh` workflow for formatting, Clippy, compilation and relevant tests. Keep warnings visible and justify narrow lint exceptions. Do not claim checks that were not run.
- Test domain rules without IO. Test Application with fake/in-memory Data Access and test SQLite at its serialization/storage boundary. Use fixed inputs and seeds where deterministic tests need them.
- Do not add CLI/GPUI dependencies to Core crates. Report what changed and which checks were run before finishing.
