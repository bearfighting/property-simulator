use std::fmt;

use crate::{AccountCategory, AccountError};

/// A four-digit chart code. Ordering is numeric and independent of posting
/// behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountCode(u16);

impl AccountCode {
    pub fn new(value: u16) -> Result<Self, AccountError> {
        if (1000..=5999).contains(&value) {
            Ok(Self(value))
        } else {
            Err(AccountError::InvalidCode(value))
        }
    }

    pub const fn value(self) -> u16 {
        self.0
    }

    pub fn validate_category(self, category: AccountCategory) -> Result<(), AccountError> {
        let (first, last) = category.code_range();
        if (first..=last).contains(&self.0) {
            Ok(())
        } else {
            Err(AccountError::CodeCategoryMismatch {
                code: self,
                category,
            })
        }
    }
}

impl fmt::Display for AccountCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_boundaries_and_rejects_outside_codes() {
        let ranges = [
            (AccountCategory::Asset, 1000, 1999),
            (AccountCategory::Liability, 2000, 2999),
            (AccountCategory::Equity, 3000, 3999),
            (AccountCategory::Income, 4000, 4999),
            (AccountCategory::Expense, 5000, 5999),
        ];
        for (category, low, high) in ranges {
            AccountCode::new(low)
                .unwrap()
                .validate_category(category)
                .unwrap();
            AccountCode::new(high)
                .unwrap()
                .validate_category(category)
                .unwrap();
        }
        for invalid in [42, 999, 6000, u16::MAX] {
            assert_eq!(
                AccountCode::new(invalid),
                Err(AccountError::InvalidCode(invalid))
            );
        }
        assert!(matches!(
            AccountCode::new(2000)
                .unwrap()
                .validate_category(AccountCategory::Asset),
            Err(AccountError::CodeCategoryMismatch { .. })
        ));
    }

    #[test]
    fn displays_as_four_digits_and_orders_numerically() {
        let first = AccountCode::new(1001).unwrap();
        assert_eq!(AccountCode::new(1000).unwrap().to_string(), "1000");
        assert!(first < AccountCode::new(1999).unwrap());
    }
}
