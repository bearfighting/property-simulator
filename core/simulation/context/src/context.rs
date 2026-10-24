use crate::{ContextError, SimulationName, UserSettings};
use property_simulation_random_engine::RandomRootKey;

/// **Incomplete:** `UserSettings` still has placeholder Slice parameters and
/// does not yet represent every opening Ledger fact. Consumers may rely on the
/// initial/latest distinction and stable system-provided root key, but not on
/// Context being a complete simulation configuration.
///
/// Complete Context when representative Slice parameters and overall settings
/// have concrete validated representations without making this crate depend on
/// the Slice execution crate.
#[derive(Clone, PartialEq, Eq)]
pub struct Context {
    simulation_name: SimulationName,
    initial_settings: UserSettings,
    latest_settings: Option<UserSettings>,
    root_key: RandomRootKey,
}

impl Context {
    /// Create an initial Context without a user override for the initial settings.
    pub fn new(
        simulation_name: SimulationName,
        root_key: RandomRootKey,
        initial_settings: UserSettings,
    ) -> Self {
        Self {
            latest_settings: None,
            simulation_name,
            initial_settings,
            root_key,
        }
    }

    /// Return a new Context with these complete settings as the latest value.
    ///
    /// Initial property, loan, and loan-term inputs cannot be changed after
    /// initialization. All other settings are replaced by the supplied value;
    /// the original Context is left unchanged.
    pub fn with_latest_settings(
        &self,
        latest_settings: UserSettings,
    ) -> Result<Self, ContextError> {
        if latest_settings.initial_values() != self.initial_settings.initial_values() {
            return Err(ContextError::InitialValuesChanged);
        }

        Ok(Self {
            simulation_name: self.simulation_name.clone(),
            initial_settings: self.initial_settings.clone(),
            latest_settings: Some(latest_settings),
            root_key: self.root_key.clone(),
        })
    }
}

impl Context {
    pub fn simulation_name(&self) -> &SimulationName {
        &self.simulation_name
    }

    pub fn initial_settings(&self) -> &UserSettings {
        &self.initial_settings
    }

    pub fn latest_settings(&self) -> Option<&UserSettings> {
        self.latest_settings.as_ref()
    }

    /// Return the latest settings when present, otherwise the initial settings.
    pub fn effective_settings(&self) -> &UserSettings {
        self.latest_settings
            .as_ref()
            .unwrap_or(&self.initial_settings)
    }

    pub fn root_key(&self) -> &RandomRootKey {
        &self.root_key
    }
}
