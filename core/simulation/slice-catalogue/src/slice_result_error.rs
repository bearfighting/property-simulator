use std::fmt;

use crate::{Effect, Event, SliceKind};

/// Failure to match a Slice result's contents to its declared kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceResultError {
    EventKindMismatch {
        slice_kind: SliceKind,
        event: Event,
    },
    EffectKindMismatch {
        slice_kind: SliceKind,
        effect: Effect,
    },
}

impl fmt::Display for SliceResultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventKindMismatch { slice_kind, event } => {
                write!(
                    f,
                    "event {event:?} does not belong to Slice kind {slice_kind:?}"
                )
            }
            Self::EffectKindMismatch { slice_kind, effect } => {
                write!(
                    f,
                    "effect {effect:?} does not belong to Slice kind {slice_kind:?}"
                )
            }
        }
    }
}

impl std::error::Error for SliceResultError {}
