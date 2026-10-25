# ADR-65: Persist pending repair cost instances and protect effective settings

- **Status:** Accepted
- **Date:** 2026-10-24
- **Related:** [ADR-57](ADR-57-per-slice-settings-persistence.md), [ADR-62](ADR-62-month-scoped-event-facts.md), [ADR-64](ADR-64-derive-slice-continuation-from-domain-facts.md), [Domain model](../domain-model.md)

## Context

A `RepairOccurred` event creates a one-time repair cost activity whose due
Month may be later than its occurrence Month. Continuing a simulation before
that due Month requires the occurrence Month, due Month, and maximum amount
that applied when the activity was created. Those facts are not reconstructible
from the generic repair settings if the settings later change.

Settings are stored as versions keyed by `effective_from_month`. Updating a
version that already affected a completed Month would change the historical
inputs inferred from that version history.

## Decision

- Add a storage-neutral `RepairCostInstanceDto` and SQLite
  `repair_cost_instances` table containing simulation, occurrence Month, due
  Month, captured maximum amount, and optional settled Month.
- In the current model, one RepairOccurrence evaluation can emit at most one
  `RepairOccurred` event per Month. Use `(simulation_id, occurrence_month)` as
  the instance key. A future behavior that allows multiple repairs in one
  Month must introduce a distinct instance identity.
- Save newly created instances and mark all due instances settled in the same
  transaction as that Month's Events and JournalEntries. The number of
  `RepairOccurred` events must match new instances; `RepairCostSettled` events
  must match due pending instances. Zero-cost settlements still count because
  they emit the settlement Event.
- Reject settings inserts, updates, or deletes whose effective Month is at or
  before any completed Month. Future settings versions remain editable until
  a completed Month uses them.

## Alternatives considered

- Recreate pending instances from the latest repair settings: rejected because
  later settings changes could alter an already generated activity.
- Persist only the repair events: rejected because the event kind and Month do
  not retain the captured cap or due Month needed to finish the activity.
- Store a separate effective-input snapshot for every Month: deferred because
  immutable, effective-month settings plus Ledger, events, and generated
  instances retain the current required inputs without duplicating a settings
  aggregate per Month.

## Consequences

- Application can reload pending repair costs and calculate them with their
  original parameters after reopening a simulation.
- Monthly accounting persistence now includes generated repair activity
  lifecycle changes in its transaction boundary.
- Effective settings can be reconstructed from version rows without allowing
  edits to rewrite completed history.
- The Core still does not own persistence records or cross-month orchestration.

## Verification examples

- A repair occurring in Month 4 with a two-month delay and a captured cap is
  stored as due in Month 6; a settings change effective in Month 5 does not
  change that instance.
- Saving Month 6 requires one `RepairCostSettled` Event for each pending
  instance due in Month 6, including zero-amount settlements.
- After Month 6 is completed, changing or deleting a settings version
  effective in Month 6 or earlier is rejected; a Month 7 version remains
  editable before Month 7 is completed.
