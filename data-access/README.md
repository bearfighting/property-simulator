# Data Access contract

This crate defines storage-neutral DTOs and the `DataAccess` persistence trait
shared by callers and adapters. It does not contain connection management or
database encodings. Each adapter maps its native rows or documents to these
types and implements the relevant store traits. `DataAccess` combines the
simulation, settings, and monthly-facts store traits for callers that need the
full contract; smaller work units can depend on an individual trait.

The public `dto` module is divided by responsibility: simulation identity and
timeline, Slice settings, stop conditions, accounting records, exact money
values, month-scoped events, and generated repair cost instances. `DataAccess`
exposes reads and writes for these persisted facts. Its async methods do not
impose a `Send` bound on returned futures; a concrete execution environment can
inform that constraint later. Monthly event and JournalEntry facts can be read
as full histories or filtered to one Month.

SQLite's schema, row mappings, and asynchronous store operations live under
`sqlite/`. Event DTOs retain month-scoped event kinds; amounts remain
Ledger-derived. Current cross-month inputs are derived from Ledger balances,
simulation months, persisted event months, and pending repair instance facts;
no Effect DTO is required.
