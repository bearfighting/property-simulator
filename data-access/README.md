# Data Access contract

This crate defines storage-neutral DTOs shared by Application and Data Access
adapters. It does not contain connection management or database encodings.
Each adapter maps its native rows or documents to these types.

The public `dto` module is divided by responsibility: simulation identity and
timeline, Slice settings, stop conditions, accounting records, exact money
values, month-scoped events, and generated repair cost instances. This is a
module within the Data Access crate, not a separate crate;
these types have no independent consumer outside that boundary today.

SQLite's schema, row mappings, and asynchronous store operations live under
`sqlite/`. Event DTOs retain month-scoped event kinds; amounts remain
Ledger-derived. Current cross-month inputs are derived from Ledger balances,
simulation months, persisted event months, and pending repair instance facts;
no Effect DTO is required.
