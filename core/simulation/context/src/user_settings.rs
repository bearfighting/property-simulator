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

impl UserSettings {
    /// Apply a typed user change and return a new complete settings value.
    ///
    /// **Incomplete:** the change variants and reconciliation rules are not
    /// defined, so this currently returns `UpdateRulesNotDefined`. Complete
    /// this method when stop-condition/Slice-setting changes preserve opening
    /// values and untouched settings, with concrete validation examples.
    pub fn apply_change(
        &self,
        _change: UserSettingsChange,
    ) -> Result<UserSettings, UserSettingsError> {
        Err(UserSettingsError::UpdateRulesNotDefined)
    }
}

/// A typed user intent to change settings.
///
/// **Incomplete:** this placeholder has no variants and carries no change data.
/// Complete it with explicit changes for representative Slice settings; do not
/// turn it into an untyped field map or a full replacement settings object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserSettingsChange {}

/// Failure to apply or validate a settings change.
///
/// **Incomplete:** only the not-yet-defined update rules are represented.
/// Complete this error type when concrete validation failures are established
/// by representative settings and callers can distinguish invalid changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserSettingsError {
    UpdateRulesNotDefined,
}

impl std::fmt::Display for UserSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpdateRulesNotDefined => {
                f.write_str("user settings update rules are not defined")
            }
        }
    }
}

impl std::error::Error for UserSettingsError {}
