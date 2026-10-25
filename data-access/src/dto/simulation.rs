use property_simulation_account::AccountCode;
use property_simulation_money::Currency;
use property_simulation_month::Month;
use property_simulation_random_engine::RandomRootKey;

use crate::DtoError;

/// Simulation identity and the currency used by all of its posted amounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationDto {
    pub id: String,
    pub name: String,
    pub currency: Currency,
}

/// Root key value for a simulation.
#[derive(Clone, PartialEq, Eq)]
pub struct SimulationRootKeyDto {
    pub simulation_id: String,
    pub root_key: [u8; 32],
}

impl SimulationRootKeyDto {
    pub fn from_domain(simulation_id: String, key: &RandomRootKey) -> Self {
        Self {
            simulation_id,
            root_key: *key.as_bytes(),
        }
    }

    pub fn from_bytes(simulation_id: String, bytes: &[u8]) -> Result<Self, DtoError> {
        let root_key = bytes
            .try_into()
            .map_err(|_| DtoError::InvalidRootKeyLength(bytes.len()))?;
        Ok(Self {
            simulation_id,
            root_key,
        })
    }

    pub fn domain_key(&self) -> RandomRootKey {
        RandomRootKey::from_bytes(self.root_key)
    }
}

/// Persisted simulation month, including initialization boundary Month 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationMonthDto {
    pub simulation_id: String,
    pub month: Month,
}

/// Per-simulation membership of a statically defined account code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationAccountDto {
    pub simulation_id: String,
    pub account_code: AccountCode,
}
