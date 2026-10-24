/// Stable semantic role that identifies an account within one account catalogue.
///
/// A role has exactly one corresponding `AccountCode` per catalogue. The role
/// is preserved as a string value for user configuration and persistence; the
/// account name remains display text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountRole(String);

impl AccountRole {
    /// Create a non-empty role key.
    pub fn new(value: impl Into<String>) -> Result<Self, crate::AccountError> {
        let value = value.into().trim().to_owned();
        if value.is_empty() {
            return Err(crate::AccountError::EmptyRole);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
