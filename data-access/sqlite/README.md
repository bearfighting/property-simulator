# SQLite schema

This directory contains the SQLite persistence adapter and schema. The
storage-neutral DTO contract is defined in the parent `data-access` crate.
The host creates the SQLx pool using `sqlite_connect_options` and injects it
into `SqliteDataAccess`; the adapter does not open a database file itself.

`schema.sql` is a persistence draft organized around stored facts, not around
the Core `Context` or `Snapshot` value objects:

- Simulation identity/name and random root are separate rows.
- A simulation's root key is write-once; repeating the same save is
  idempotent, while attempting to replace it is rejected.
- There is no persisted aggregate `UserSettings` table. Each static Slice kind
  has a typed settings table, keyed by simulation and effective starting month.
  Current scope supports one loan configuration per simulation/month. Loan
  settings distinguish the remaining term at simulation start from the
  dynamic remaining term passed to each month's calculation. Loan terms are
  stored with Loan settings, not with opening Ledger facts.
  Slice settings begin at Month 1 because Month 0 is initialization and has no
  Slice calculation. Later rows replace that Slice's configuration from the
  specified month onward. The greatest applicable effective month is the
  latest value. An all-NULL row is a tombstone that removes a prior
  configuration. Repeated edits before the same month replace that month's row.
- Stop conditions are independent simulation-level values, versioned by
  effective month. `stop_condition_sets` records that a complete set was
  supplied, including an empty set; `stop_conditions` stores its members.
- There is no Context table. Application reads the root key, current per-Slice
  settings, and current stop conditions as separate simulation facts.
- `simulation_months` records the simulation timeline: Month 0 is the
  initialized boundary, and each Month 1+ row represents a completed period.
  Its `(simulation_id, month)` key anchors month-scoped records.
- Currency belongs to the simulation. `simulation_accounts` records the
  account codes used by that simulation; Application still supplies the
  authoritative account catalogue, while this table provides membership and
  referential integrity for journal lines.
- Events and JournalEntries reference the simulation's Month row.
  Month 0 supports one or more initialization JournalEntries; Month 1+ rows represent
  completed periods. Effective settings for a completed month are the latest
  per-Slice settings rows whose effective month is less than or equal to that
  Month. Settings may be saved for a future, not-yet-completed Month, so their
  effective month is not a foreign key to `simulation_months`. No separate
  Context, Snapshot, or Ledger entity table is required.
- Generated repair cost instances retain their occurrence Month, due Month,
  and maximum amount captured when the occurrence created the instance. Their
  due-month settlement and the month's events and JournalEntries are saved in
  one transaction. The current model permits at most one repair occurrence
  per simulation Month, so occurrence Month is the instance key.
- Schema triggers reject inserting, updating, or deleting settings effective
  at or before a completed Month. This preserves the configuration history
  needed to reconstruct the effective settings used by earlier Months.
- JournalEntries are stored once as month-scoped accounting facts. Core Ledger
  is reconstructed by ordering the simulation's entries; balances are derived
  from those entries and are not stored as competing facts.
- Journal lines store exact decimal coefficient/scale values as text/integer,
  avoiding SQLite floating-point numeric storage.
- Events are rows containing only their completed Month and event kind. The
  event ordinal preserves repeated events and order within that Month; it is a
  storage position, not a domain or Slice instance ID. Event kinds do not
  duplicate SliceKind, event amounts, or Ledger source information.
- JournalEntries are stored once in the journal tables. Empty Slice results need
  no row because they contain no events or entries.

This schema does not promise cross-version compatibility or introduce a
migration/versioning policy.

The database cannot enforce that each JournalEntry balances across its lines.
The adapter validates entries through Core before storing an accounting month.
The accounting-month operation stores the Month, month-scoped Events, and
JournalEntries.

`Month` is represented as `u16` in Core and DTOs. SQLite stores it as an
INTEGER with the semantic lower-bound check only; row decoding rejects negative
or out-of-range values. The schema does not duplicate the Rust upper bound.

The shared DTO contract covers Simulation, root key, exact money values,
per-Slice settings, stop conditions, Month/account membership, grouped
journal entries, and month-scoped event collections.
