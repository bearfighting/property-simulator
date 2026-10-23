/// Optional machine-readable account purpose supplied with the account definition.
///
/// The value is an opaque key. Reporting rules decide which known keys they
/// understand; account names remain presentation labels and are not parsed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountRole(String);

impl AccountRole {
    /// Create a non-empty role key.
    pub fn new(value: impl Into<String>) -> Result<Self, crate::AccountError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(crate::AccountError::EmptyRole);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
