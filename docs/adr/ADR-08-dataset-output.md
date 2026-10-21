# ADR-08: Dataset as core output

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md)
- **Test vectors:** N/A; in-memory dataset invariants are tested by the simulation/archive tests. Serialization and export tests belong to Phase II.

## Context
Core results need to remain analyzable and usable by multiple future interfaces.

## Decision
The core product is a complete in-memory dataset of configuration, provenance, completed results, snapshots, and branch relationships. M5 defines this runtime data contract only; it does not promise cross-upgrade API compatibility. JSON is a possible full exchange representation; CSV/XLSX may provide flattened exports in Phase II. These internal project formats do not define a general external schema or cross-upgrade compatibility contract. Do not generate investment recommendations or prebuilt investment reports.

## Alternatives considered
- UI-specific result objects or report-first output: couples the core to presentation and interpretation.

## Consequences
Keep configuration, provenance, events, effects, transactions, and snapshots explicit in runtime data. When an application-level dataset is introduced, record the application version once for the dataset/run; do not attach independent versions to models or random streams. SQLite and export representations are deferred to Phase II; no unfinished-run random state is part of the dataset.
