use crate::DomainError;
use std::{fmt, str::FromStr};

/// A valid calendar month in the supported Gregorian year range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    year: u16,
    month: u8,
}

impl YearMonth {
    pub fn new(year: u16, month: u8) -> Result<Self, DomainError> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return Err(DomainError::InvalidYearMonth);
        }
        Ok(Self { year, month })
    }

    pub const fn year(self) -> u16 {
        self.year
    }
    pub const fn month(self) -> u8 {
        self.month
    }

    pub fn checked_add_months(self, delta: i32) -> Result<Self, DomainError> {
        let absolute = i64::from(self.year) * 12 + i64::from(self.month) - 1 + i64::from(delta);
        if !(12..=119_999).contains(&absolute) {
            return Err(DomainError::MonthOutOfRange);
        }
        Self::new((absolute / 12) as u16, (absolute % 12 + 1) as u8)
    }

    pub fn checked_sub_months(self, delta: i32) -> Result<Self, DomainError> {
        self.checked_add_months(delta.checked_neg().ok_or(DomainError::MonthOutOfRange)?)
    }

    pub fn range_inclusive(self, end: Self) -> Result<YearMonthRange, DomainError> {
        if self > end {
            return Err(DomainError::InvalidYearMonth);
        }
        Ok(YearMonthRange {
            next: Some(self),
            end,
        })
    }
}

impl fmt::Display for YearMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

impl FromStr for YearMonth {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (year, month) = value.split_once('-').ok_or(DomainError::InvalidYearMonth)?;
        if year.len() != 4 || month.len() != 2 {
            return Err(DomainError::InvalidYearMonth);
        }
        Self::new(
            year.parse().map_err(|_| DomainError::InvalidYearMonth)?,
            month.parse().map_err(|_| DomainError::InvalidYearMonth)?,
        )
    }
}

pub struct YearMonthRange {
    next: Option<YearMonth>,
    end: YearMonth,
}

impl Iterator for YearMonthRange {
    type Item = YearMonth;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;
        self.next = if current == self.end {
            None
        } else {
            current.checked_add_months(1).ok()
        };
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_formats_orders_and_moves_across_years() {
        let jan: YearMonth = "2025-01".parse().unwrap();
        assert_eq!(jan.to_string(), "2025-01");
        assert!("2025-13".parse::<YearMonth>().is_err());
        assert!("25-01".parse::<YearMonth>().is_err());
        assert_eq!(jan.checked_sub_months(1).unwrap().to_string(), "2024-12");
        assert_eq!(jan.checked_add_months(11).unwrap().to_string(), "2025-12");
        assert!(YearMonth::new(0, 1).is_err());
        assert!(YearMonth::new(2025, 0).is_err());
        assert!(
            YearMonth::new(9999, 12)
                .unwrap()
                .checked_add_months(1)
                .is_err()
        );
        assert!(YearMonth::new(2024, 1).unwrap() < jan);
    }

    #[test]
    fn inclusive_range_includes_both_ends() {
        let months: Vec<_> = YearMonth::new(2024, 12)
            .unwrap()
            .range_inclusive(YearMonth::new(2025, 2).unwrap())
            .unwrap()
            .collect();
        assert_eq!(
            months.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["2024-12", "2025-01", "2025-02"]
        );
    }
}
