# ADR-08: Dataset as core output

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related:** [domain model](../domain-model.md)
- **Test vectors:** N/A; serialization contract tests arrive with M5.

## Context
Core results need to remain analyzable and usable by multiple future interfaces.

## Decision
The core product is a complete versioned dataset. JSON is the full exchange representation; CSV/XLSX may provide flattened exports. Do not generate investment recommendations or prebuilt investment reports.

## Alternatives considered
- UI-specific result objects or report-first output: couples the core to presentation and interpretation.

## Consequences
Persist configuration, provenance, events, effects, transactions, snapshots, and schema/model versions explicitly.
