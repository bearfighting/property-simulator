use std::collections::HashMap;

use crate::{SliceKind, SliceSetting};

/// A collection of per-kind settings with keys derived from their enum variants.
///
/// The map is private so callers cannot pair a `SliceKind` with a different
/// kind's settings. Inserting a setting replaces the prior value for that kind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SliceSettings {
    by_kind: HashMap<SliceKind, SliceSetting>,
}

impl SliceSettings {
    /// Insert or replace the settings for their encoded Slice kind.
    pub fn insert(&mut self, setting: SliceSetting) -> Option<SliceSetting> {
        self.by_kind.insert(setting.kind(), setting)
    }

    /// Return settings for a Slice kind, if present.
    pub fn get(&self, kind: SliceKind) -> Option<&SliceSetting> {
        self.by_kind.get(&kind)
    }

    /// Remove and return settings for a Slice kind, if present.
    pub fn remove(&mut self, kind: SliceKind) -> Option<SliceSetting> {
        self.by_kind.remove(&kind)
    }

    /// Iterate over configured settings in unspecified order.
    pub fn iter(&self) -> impl Iterator<Item = &SliceSetting> {
        self.by_kind.values()
    }
}
