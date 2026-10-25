use std::fmt;

/// Failure while reading or writing SQLite persistence data.
#[derive(Debug)]
pub enum SqliteDataAccessError {
    Database(sqlx::Error),
    InvalidData(&'static str),
    Dto(property_simulation_data_access::DtoError),
}

impl fmt::Display for SqliteDataAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => write!(f, "SQLite operation failed: {error}"),
            Self::InvalidData(field) => write!(f, "invalid SQLite value for {field}"),
            Self::Dto(error) => write!(f, "invalid Data Access DTO: {error}"),
        }
    }
}

impl std::error::Error for SqliteDataAccessError {}

impl From<SqliteDataAccessError> for property_simulation_data_access::DataAccessError {
    fn from(error: SqliteDataAccessError) -> Self {
        Self::storage(error.to_string())
    }
}

impl From<sqlx::Error> for SqliteDataAccessError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<property_simulation_data_access::DtoError> for SqliteDataAccessError {
    fn from(error: property_simulation_data_access::DtoError) -> Self {
        Self::Dto(error)
    }
}

impl From<property_simulation_money::MoneyError> for SqliteDataAccessError {
    fn from(error: property_simulation_money::MoneyError) -> Self {
        Self::Dto(property_simulation_data_access::DtoError::Money(error))
    }
}
