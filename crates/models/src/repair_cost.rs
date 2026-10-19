use property_simulation_domain::{DomainError, Money};

/// A bounded discrete repair-cost distribution. The inclusive upper bound makes
/// every supported sample safe to validate before a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairCostDistribution {
    pub maximum: Money,
}

impl RepairCostDistribution {
    pub fn validate_support(&self) -> Result<(), DomainError> {
        if self.maximum.is_negative() {
            return Err(DomainError::InvalidMoneyAmount);
        }
        Ok(())
    }

    pub fn validate_sample(&self, value: Money) -> Result<(), DomainError> {
        self.validate_support()?;
        if value.is_negative()
            || value.currency() != self.maximum.currency()
            || value.checked_cmp(self.maximum)? == std::cmp::Ordering::Greater
        {
            return Err(DomainError::InvalidMoneyAmount);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_repair_support_includes_edges_and_rejects_outside() {
        let cad = property_simulation_domain::Currency::from_code("CAD").unwrap();
        let d = RepairCostDistribution {
            maximum: Money::parse("50", cad).unwrap(),
        };
        assert!(d.validate_sample(Money::parse("0", cad).unwrap()).is_ok());
        assert!(d.validate_sample(d.maximum).is_ok());
        assert!(
            d.validate_sample(Money::parse("50.01", cad).unwrap())
                .is_err()
        );
    }
}
