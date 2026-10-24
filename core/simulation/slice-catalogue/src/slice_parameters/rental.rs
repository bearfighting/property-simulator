use property_simulation_money::Money;
use std::fmt;

/// Settings for a fixed rental amount collected in each calculated month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RentalParameters {
    monthly_rent: Money,
}

impl RentalParameters {
    /// Create rental settings. Zero rent is allowed and produces no receipt;
    /// negative rent is invalid.
    pub fn new(monthly_rent: Money) -> Result<Self, RentalParametersError> {
        if monthly_rent.is_negative() {
            return Err(RentalParametersError::NegativeMonthlyRent);
        }

        Ok(Self { monthly_rent })
    }
}

impl RentalParameters {
    pub const fn monthly_rent(&self) -> Money {
        self.monthly_rent
    }
}

/// Failure to construct rental settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RentalParametersError {
    NegativeMonthlyRent,
}

impl fmt::Display for RentalParametersError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeMonthlyRent => f.write_str("monthly rent cannot be negative"),
        }
    }
}

impl std::error::Error for RentalParametersError {}
