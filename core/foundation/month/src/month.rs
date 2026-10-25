use std::fmt;

/// A zero-based month in a simulation.
///
/// `Month::initial()` is the simulation initialization boundary. Month 1 is the
/// first calculated month. The value is a key within one simulation, not a
/// globally unique identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month(u16);

/// Failure while performing month arithmetic or constructing a month range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonthError {
    OutOfRange,
    EarlierMonth,
    StartAfterEnd,
}

impl Month {
    /// The initialization boundary before the first calculated month.
    pub const fn initial() -> Self {
        Self(0)
    }

    /// The first calculated month in a simulation.
    pub const fn first_calculated() -> Self {
        Self(1)
    }

    /// Whether this value represents the initialization boundary.
    pub const fn is_initial(self) -> bool {
        self.0 == 0
    }

    /// Construct a zero-based simulation month.
    pub const fn new(number: u16) -> Self {
        Self(number)
    }

    /// Return this month's zero-based position in the simulation.
    pub const fn number(self) -> u16 {
        self.0
    }
}

impl Month {
    /// Return the next simulation month, or an error if it would overflow.
    pub const fn next(self) -> Result<Self, MonthError> {
        self.checked_add(1)
    }

    /// Move forward by a number of months, checking for overflow.
    pub const fn checked_add(self, months: u16) -> Result<Self, MonthError> {
        match self.0.checked_add(months) {
            Some(number) => Ok(Self(number)),
            None => Err(MonthError::OutOfRange),
        }
    }

    /// Move backward by a number of months, checking for underflow.
    pub const fn checked_sub(self, months: u16) -> Result<Self, MonthError> {
        match self.0.checked_sub(months) {
            Some(number) => Ok(Self(number)),
            None => Err(MonthError::OutOfRange),
        }
    }

    /// Return the number of months elapsed since `earlier`.
    pub const fn months_since(self, earlier: Self) -> Result<u16, MonthError> {
        match self.0.checked_sub(earlier.0) {
            Some(months) => Ok(months),
            None => Err(MonthError::EarlierMonth),
        }
    }
}

impl Month {
    /// Create a range that includes both `self` and `end`.
    pub const fn range_inclusive(self, end: Self) -> Result<MonthRange, MonthError> {
        if self.0 > end.0 {
            Err(MonthError::StartAfterEnd)
        } else {
            Ok(MonthRange {
                next: Some(self),
                end,
            })
        }
    }
}

/// Inclusive sequence of simulation months.
pub struct MonthRange {
    next: Option<Month>,
    end: Month,
}

impl Iterator for MonthRange {
    type Item = Month;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;
        self.next = if current == self.end {
            None
        } else {
            current.next().ok()
        };
        Some(current)
    }
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for MonthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange => f.write_str("simulation month is outside the supported range"),
            Self::EarlierMonth => f.write_str("month precedes the supplied earlier month"),
            Self::StartAfterEnd => f.write_str("month range start is after its end"),
        }
    }
}

impl std::error::Error for MonthError {}
