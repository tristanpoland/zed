//! Backend-neutral accessibility types and platform capability contracts.

pub use accesskit::{
    Action, ActionData, ActionRequest, Affine, Node, NodeId, NodeIdContent, Orientation, Point,
    Rect, Role, Size, TextPosition, TextSelection, Toggled, Tree, TreeId, TreeUpdate, Vec2,
};

/// The AccessKit action type used by GPUI's accessibility APIs.
pub type AccessibleAction = Action;

/// Callbacks used by a platform accessibility adapter.
pub struct A11yCallbacks {
    /// Called when the adapter is activated (a screen reader connects).
    pub activation: Box<dyn Fn() -> Option<TreeUpdate> + Send + 'static>,
    /// Called when an action is requested by the screen reader.
    pub action: Box<dyn Fn(ActionRequest) + Send + 'static>,
    /// Called when the adapter is deactivated (screen reader disconnects).
    pub deactivation: Box<dyn Fn() + Send + 'static>,
}

/// Accessibility operations provided by a platform window implementation.
pub trait PlatformAccessibilitySpi {
    /// Initialize the platform accessibility adapter with callbacks.
    fn a11y_init(&self, callbacks: A11yCallbacks);

    /// Provide a tree update to the platform accessibility adapter.
    fn a11y_tree_update(&self, tree_update: TreeUpdate);

    /// Inform the adapter of updated window bounds.
    fn a11y_update_window_bounds(&self);
}
