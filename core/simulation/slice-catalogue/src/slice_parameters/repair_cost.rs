use std::{fmt, num::NonZeroU32};

use property_simulation_money::Money;

/// Settings shared by generated one-time repair cost instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairCostParameters {
    maximum_amount: Money,
    delay_months: NonZeroU32,
}

impl RepairCostParameters {
    pub fn new(
        maximum_amount: Money,
        delay_months: NonZeroU32,
    ) -> Result<Self, RepairCostParametersError> {
        if maximum_amount.is_negative() {
            return Err(RepairCostParametersError::NegativeMaximumAmount);
        }
        if u16::try_from(delay_months.get()).is_err() {
            return Err(RepairCostParametersError::DelayOutOfRange);
        }

        Ok(Self {
            maximum_amount,
            delay_months,
        })
    }

    pub const fn maximum_amount(self) -> Money {
        self.maximum_amount
    }

    pub const fn delay_months(self) -> NonZeroU32 {
        self.delay_months
    }
}

/// Invalid repair cost configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairCostParametersError {
    NegativeMaximumAmount,
    DelayOutOfRange,
}

impl fmt::Display for RepairCostParametersError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeMaximumAmount => f.write_str("maximum repair cost cannot be negative"),
            Self::DelayOutOfRange => {
                f.write_str("repair cost delay exceeds the supported month range")
            }
        }
    }
}

impl std::error::Error for RepairCostParametersError {}
