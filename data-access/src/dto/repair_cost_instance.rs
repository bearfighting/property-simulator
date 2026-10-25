use property_simulation_month::Month;

use crate::{DtoError, dto::MoneyDto};

/// A generated repair cost that is due in a future month or has settled.
///
/// The current repair occurrence Slice produces at most one instance per
/// Month, so `(simulation_id, occurrence_month)` is its storage identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairCostInstanceDto {
    pub simulation_id: String,
    pub occurrence_month: Month,
    pub due_month: Month,
    pub maximum_amount: MoneyDto,
    /// The due Month once its settlement event and accounting result are saved.
    pub settled_month: Option<Month>,
}

impl RepairCostInstanceDto {
    /// Validate this instance and associate it with its occurrence Month.
    pub fn validate(&self) -> Result<(), DtoError> {
        let maximum_amount = self
            .maximum_amount
            .to_domain(self.maximum_amount.currency)?;
        if self.occurrence_month.is_initial()
            || self.due_month.is_initial()
            || self.due_month <= self.occurrence_month
            || maximum_amount.is_negative()
            || self
                .settled_month
                .is_some_and(|month| month != self.due_month)
        {
            return Err(DtoError::InvalidRepairCostInstance);
        }
        Ok(())
    }
}
