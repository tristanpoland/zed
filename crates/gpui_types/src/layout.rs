//! Backend-neutral identities used by GPUI's layout protocol.

/// An identifier for a node in a window's layout tree.
///
/// The layout engine that owns the tree is backend-specific. This identity is
/// shared so the app-facing element and window APIs do not expose that engine's
/// node type.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct LayoutId(u64);

impl LayoutId {
    /// Creates an identifier from the backend's stable raw representation.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the backend's stable raw representation.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for LayoutId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<usize> for LayoutId {
    fn from(value: usize) -> Self {
        Self::new(value as u64)
    }
}

impl From<LayoutId> for u64 {
    fn from(value: LayoutId) -> Self {
        value.as_u64()
    }
}

impl From<LayoutId> for usize {
    fn from(value: LayoutId) -> Self {
        value.as_u64() as usize
    }
}
