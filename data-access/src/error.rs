use std::fmt;

/// Invalid DTO data or a failed conversion between a DTO and Core.
#[derive(Debug)]
pub enum DtoError {
    InvalidRootKeyLength(usize),
    InvalidRepairCostInstance,
    Money(property_simulation_money::MoneyError),
    JournalEntry(property_simulation_journal_entry::JournalEntryError),
}

impl fmt::Display for DtoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRootKeyLength(length) => {
                write!(f, "root key must contain 32 bytes, got {length}")
            }
            Self::InvalidRepairCostInstance => {
                f.write_str("invalid generated repair cost instance")
            }
            Self::Money(error) => write!(f, "invalid DTO money: {error}"),
            Self::JournalEntry(error) => write!(f, "invalid DTO journal entry: {error}"),
        }
    }
}

impl std::error::Error for DtoError {}

/// Failure reported by a Data Access adapter without exposing backend types.
#[derive(Debug)]
pub struct DataAccessError {
    message: String,
}

impl DataAccessError {
    /// Create a storage failure from an adapter-provided diagnostic message.
    pub fn storage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DataAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "data access operation failed: {}", self.message)
    }
}

impl std::error::Error for DataAccessError {}

impl From<property_simulation_money::MoneyError> for DtoError {
    fn from(error: property_simulation_money::MoneyError) -> Self {
        Self::Money(error)
    }
}

impl From<property_simulation_journal_entry::JournalEntryError> for DtoError {
    fn from(error: property_simulation_journal_entry::JournalEntryError) -> Self {
        Self::JournalEntry(error)
    }
}
