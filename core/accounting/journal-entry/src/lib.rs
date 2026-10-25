//! Balanced accounting entries shared by initialization, domain Slices, and Ledger.

mod error;
mod journal_entry;
mod journal_line;

pub use error::JournalEntryError;
pub use journal_entry::JournalEntry;
pub use journal_line::JournalLine;
