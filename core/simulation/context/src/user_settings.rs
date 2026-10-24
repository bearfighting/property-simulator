use property_simulation_slice_catalogue::SliceSettings;

use crate::{InitialValues, StopCondition};

/// User-owned simulation settings shared through Context.
///
/// **Incomplete:** opening equity is derived, but this type does not yet
/// contain every account opening fact; most Slice parameter structs are also
/// placeholders. Consumers must not treat it as a complete or fully validated
/// simulation configuration.
///
/// Complete it when remaining opening facts and concrete Slice parameter and
/// whole-settings validation rules are agreed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSettings {
    initial_values: InitialValues,
    stop_conditions: Vec<StopCondition>,
    slice_settings: SliceSettings,
}

impl UserSettings {
    /// Create user settings from opening values, stop conditions, and Slice settings.
    pub fn new(
        initial_values: InitialValues,
        stop_conditions: Vec<StopCondition>,
        slice_settings: SliceSettings,
    ) -> Self {
        Self {
            initial_values,
            stop_conditions,
            slice_settings,
        }
    }
}

impl UserSettings {
    /// Return the immutable opening values supplied for this simulation.
    pub const fn initial_values(&self) -> &InitialValues {
        &self.initial_values
    }

    /// Return the stop conditions; any satisfied condition stops the simulation.
    pub fn stop_conditions(&self) -> &[StopCondition] {
        &self.stop_conditions
    }

    /// Return the current per-Slice settings.
    pub fn slice_settings(&self) -> &SliceSettings {
        &self.slice_settings
    }
}
