//! Backend-neutral values used by GPUI's paint protocol.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The ordering assigned to a paint operation in a scene.
pub type DrawOrder = u32;

/// A boolean stored as a `u32` so GPU-facing structs contain no compiler-inserted
/// padding bytes. Guaranteed to be `0` or `1` by construction.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct PaddedBool32(u32);

impl From<bool> for PaddedBool32 {
    fn from(value: bool) -> Self {
        Self(value as u32)
    }
}

/// The style of a border.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[repr(C)]
pub enum BorderStyle {
    /// A solid border.
    #[default]
    Solid = 0,
    /// A dashed border.
    Dashed = 1,
}
