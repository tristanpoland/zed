//! Backend-neutral data types used by GPUI APIs and implementations.

use std::any::{Any, TypeId};

/// The entity-storage capability required by a GPUI application context.
///
/// The state operations are type-erased so implementations can provide their
/// own storage without depending on GPUI. Typed callbacks still run through
/// the public GPUI context methods so they can receive the implementation's
/// real context type.
pub trait EntityStorageSpi {
    /// Returns whether an entity with the given identifier is stored.
    fn contains(&self, entity_id: EntityId) -> bool;

    /// Returns the concrete type stored for an entity, if it exists.
    fn entity_type(&self, entity_id: EntityId) -> Option<TypeId>;

    /// Reserves an entity identifier and its handle lifetime for a later insertion.
    fn reserve(&self) -> EntityReservation;

    /// Inserts type-erased state into a previously reserved entity slot.
    fn insert(&mut self, reservation: EntityReservation, entity: Box<dyn Any>) -> EntityId;

    /// Temporarily removes type-erased state so an entity can be updated.
    fn lease(&mut self, entity_id: EntityId) -> Option<Box<dyn Any>>;

    /// Returns type-erased state for a read-only entity operation.
    fn read(&self, entity_id: EntityId) -> Option<&dyn Any>;

    /// Returns leased state to the storage after an update.
    fn end_lease(&mut self, entity_id: EntityId, entity: Box<dyn Any>);
}

/// A token for an entity slot reserved by an [`EntityStorageSpi`].
#[derive(Clone, Copy, Debug)]
pub struct EntityReservation {
    entity_id: EntityId,
}

impl EntityReservation {
    /// Creates a reservation for an entity identifier.
    pub fn new(entity_id: EntityId) -> Self {
        Self { entity_id }
    }

    /// Returns the identifier associated with this reservation.
    pub fn entity_id(self) -> EntityId {
        self.entity_id
    }
}

/// The application capability required by a GPUI context implementation.
///
/// The implementation owns the entity storage and remains free to choose its
/// concrete handle types. A backend can implement this trait without
/// importing `gpui`.
pub trait AppContextSpi {
    /// Returns the entity-storage capability for this application context.
    fn entity_storage(&self) -> &dyn EntityStorageSpi;

    /// Returns the concrete type stored for an entity, if it exists.
    fn entity_type(&self, entity_id: EntityId) -> Option<TypeId> {
        self.entity_storage().entity_type(entity_id)
    }

    /// Returns whether an entity with the given identifier is currently stored
    /// in this application context.
    fn entity_exists(&self, entity_id: EntityId) -> bool {
        self.entity_storage().contains(entity_id)
    }

    /// Reserves an entity identifier and its handle lifetime for a later insertion.
    fn reserve_entity(&self) -> EntityReservation {
        self.entity_storage().reserve()
    }

    /// Reads type-erased state for an entity operation.
    fn read_entity(&self, entity_id: EntityId) -> Option<&dyn Any> {
        self.entity_storage().read(entity_id)
    }
}

/// The common contract exposed by every entity handle.
pub trait EntityHandle {
    /// Returns the identifier of the referenced entity.
    fn entity_id(&self) -> EntityId;
}

/// The contract exposed by a strong, typed entity handle.
pub trait StrongEntityHandle<T>: EntityHandle {
    /// The corresponding weak handle type.
    type Weak: WeakEntityHandle<T>;

    /// Downgrades this handle without changing the entity's lifetime.
    fn downgrade(&self) -> Self::Weak;
}

/// The contract exposed by a weak, typed entity handle.
pub trait WeakEntityHandle<T>: EntityHandle {
    /// The corresponding strong handle type.
    type Strong: StrongEntityHandle<T>;

    /// Attempts to upgrade this handle.
    fn upgrade(&self) -> Option<Self::Strong>;
}

/// The cancellation contract exposed by a subscription.
pub trait SubscriptionHandle {
    /// Detaches the subscription while preserving its callback.
    fn detach(self);
}

/// The cancellation contract exposed by a scheduled task.
pub trait TaskHandle<T> {
    /// Detaches the task so it runs independently of this handle.
    fn detach(self);
}

/// A global value that can be stored in an application context.
pub trait Global: 'static {}

/// Associates an entity type with an event type it can emit.
pub trait EventEmitter<E: Any>: 'static {}

pub mod geometry {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use std::fmt::Debug;

    /// An axis in two-dimensional space.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
    pub enum Axis {
        /// The horizontal axis.
        #[default]
        Horizontal,
        /// The vertical axis.
        Vertical,
    }

    /// A location in two-dimensional space.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
    pub struct Point<T> {
        /// The horizontal coordinate.
        pub x: T,
        /// The vertical coordinate.
        pub y: T,
    }

    /// A two-dimensional size.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
    pub struct Size<T> {
        /// The width.
        pub width: T,
        /// The height.
        pub height: T,
    }

    /// A rectangular area in two-dimensional space.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
    pub struct Bounds<T> {
        /// The top-left origin.
        pub origin: Point<T>,
        /// The extent of the area.
        pub size: Size<T>,
    }

    /// Insets on the four sides of a rectangle.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
    pub struct Edges<T> {
        /// The top inset.
        pub top: T,
        /// The right inset.
        pub right: T,
        /// The bottom inset.
        pub bottom: T,
        /// The left inset.
        pub left: T,
    }

    /// An angle in radians.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
    pub struct Radians(pub f32);

    /// A fractional percentage.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
    pub struct Percentage(pub f32);

    /// Logical pixels.
    #[derive(
        Copy, Clone, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
    )]
    pub struct Pixels(pub f32);

    /// Device pixels.
    #[derive(
        Copy,
        Clone,
        Debug,
        Default,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Serialize,
        Deserialize,
        JsonSchema,
    )]
    pub struct DevicePixels(pub i32);

    /// Scaled logical pixels used by the paint protocol.
    #[derive(
        Copy, Clone, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
    )]
    pub struct ScaledPixels(pub f32);

    /// Root-relative units.
    #[derive(
        Copy, Clone, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
    )]
    pub struct Rems(pub f32);

    /// Construct a point.
    pub const fn point<T>(x: T, y: T) -> Point<T> {
        Point { x, y }
    }

    /// Construct a size.
    pub const fn size<T>(width: T, height: T) -> Size<T> {
        Size { width, height }
    }

    /// Construct a bounds value.
    pub const fn bounds<T>(origin: Point<T>, size: Size<T>) -> Bounds<T> {
        Bounds { origin, size }
    }

    /// Construct logical pixels.
    pub const fn px(value: f32) -> Pixels {
        Pixels(value)
    }

    /// Construct scaled logical pixels.
    pub const fn scaled(value: f32) -> ScaledPixels {
        ScaledPixels(value)
    }

    /// Construct root-relative units.
    pub const fn rems(value: f32) -> Rems {
        Rems(value)
    }

    impl<T> Point<T> {
        /// Map both coordinates to another type.
        pub fn map<U>(&self, f: impl Fn(&T) -> U) -> Point<U> {
            Point {
                x: f(&self.x),
                y: f(&self.y),
            }
        }
    }

    impl<T> Size<T> {
        /// Map both dimensions to another type.
        pub fn map<U>(&self, f: impl Fn(&T) -> U) -> Size<U> {
            Size {
                width: f(&self.width),
                height: f(&self.height),
            }
        }
    }

    impl<T> Bounds<T> {
        /// Construct bounds from an origin and size.
        pub const fn new(origin: Point<T>, size: Size<T>) -> Self {
            Self { origin, size }
        }
    }

    impl<T: Clone + Debug> Edges<T> {
        /// Construct equal insets on every side.
        pub fn all(value: T) -> Self {
            Self {
                top: value.clone(),
                right: value.clone(),
                bottom: value.clone(),
                left: value,
            }
        }
    }
}

pub mod color {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    /// An RGBA color with normalized components.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
    pub struct Rgba {
        /// Red component.
        pub r: f32,
        /// Green component.
        pub g: f32,
        /// Blue component.
        pub b: f32,
        /// Alpha component.
        pub a: f32,
    }

    /// An HSLA color with normalized components.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
    pub struct Hsla {
        /// Hue, in degrees.
        pub h: f32,
        /// Saturation.
        pub s: f32,
        /// Lightness.
        pub l: f32,
        /// Alpha component.
        pub a: f32,
    }

    /// A color space used by paint operations.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
    pub enum ColorSpace {
        /// sRGB.
        #[default]
        Srgb,
        /// Display-P3.
        DisplayP3,
    }

    /// Construct an opaque RGB color from a hexadecimal value.
    pub const fn rgb(hex: u32) -> Rgba {
        Rgba {
            r: ((hex >> 16) & 0xff) as f32 / 255.0,
            g: ((hex >> 8) & 0xff) as f32 / 255.0,
            b: (hex & 0xff) as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Construct an RGBA color from a hexadecimal value.
    pub const fn rgba(hex: u32) -> Rgba {
        Rgba {
            r: ((hex >> 24) & 0xff) as f32 / 255.0,
            g: ((hex >> 16) & 0xff) as f32 / 255.0,
            b: ((hex >> 8) & 0xff) as f32 / 255.0,
            a: (hex & 0xff) as f32 / 255.0,
        }
    }

    /// Construct an HSLA color.
    pub const fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
        Hsla { h, s, l, a }
    }
}

pub mod input {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    /// The state of the modifier keys.
    #[derive(
        Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
    )]
    pub struct Modifiers {
        /// Control is pressed.
        pub control: bool,
        /// Alt is pressed.
        pub alt: bool,
        /// Shift is pressed.
        pub shift: bool,
        /// The platform modifier is pressed.
        pub platform: bool,
        /// Function is pressed.
        pub function: bool,
    }

    impl Modifiers {
        /// Whether any modifier is pressed.
        pub const fn modified(self) -> bool {
            self.control || self.alt || self.shift || self.platform || self.function
        }
    }

    /// The state of the caps lock key.
    #[derive(
        Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
    )]
    pub struct Capslock {
        /// Whether caps lock is enabled.
        pub on: bool,
    }

    /// An identifier for an active touch contact.
    #[derive(
        Copy,
        Clone,
        Debug,
        Default,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        Serialize,
        Deserialize,
        JsonSchema,
    )]
    pub struct TouchId(pub u64);
}

pub mod platform {
    pub use crate::application::{AppLifecyclePhase, PlatformApplicationSpi};
    pub use crate::clipboard::{
        ClipboardEntry, ClipboardImage, ClipboardItem, ClipboardReadError, ClipboardString,
        ExternalPaths, ImageFormat, PlatformClipboardSpi,
    };
    pub use crate::credentials::PlatformCredentialsSpi;
    pub use crate::notifications::{
        PlatformSystemNotificationSpi, SystemNotification, SystemNotificationAction,
        SystemNotificationResponse,
    };
    pub use crate::urls::PlatformUrlSpi;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use std::fmt;

    /// An opaque identifier for a hardware display.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DisplayId(pub u64);

    /// An identifier for an image resource.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct ImageId(pub usize);

    /// An identifier for a paint path.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct PathId(pub usize);

    /// The style of the cursor (pointer).
    #[derive(
        Copy, Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
    )]
    pub enum CursorStyle {
        /// The default cursor.
        #[default]
        Arrow,

        /// A text input cursor.
        IBeam,

        /// A crosshair cursor.
        Crosshair,

        /// A closed hand cursor.
        ClosedHand,

        /// An open hand cursor.
        OpenHand,

        /// A pointing hand cursor.
        PointingHand,

        /// A resize left cursor.
        ResizeLeft,

        /// A resize right cursor.
        ResizeRight,

        /// A resize cursor to the left and right.
        ResizeLeftRight,

        /// A resize up cursor.
        ResizeUp,

        /// A resize down cursor.
        ResizeDown,

        /// A resize cursor directing up and down.
        ResizeUpDown,

        /// A resize cursor directing up-left and down-right.
        ResizeUpLeftDownRight,

        /// A resize cursor directing up-right and down-left.
        ResizeUpRightDownLeft,

        /// A cursor indicating that the item/column can be resized horizontally.
        ResizeColumn,

        /// A cursor indicating that the item/row can be resized vertically.
        ResizeRow,

        /// A text input cursor for vertical layout.
        IBeamCursorForVerticalLayout,

        /// A cursor indicating that the operation is not allowed.
        OperationNotAllowed,

        /// A cursor indicating that the operation will result in a link.
        DragLink,

        /// A cursor indicating that the operation will result in a copy.
        DragCopy,

        /// A cursor indicating that the operation will result in a context menu.
        ContextualMenu,
    }

    /// Cursor operations supplied by a platform implementation.
    pub trait PlatformCursorSpi {
        /// Sets the cursor style for the active application window.
        fn set_cursor_style(&self, style: CursorStyle);

        /// Hides the cursor until the user moves the mouse over an application window.
        fn hide_cursor_until_mouse_moves(&self);

        /// Returns whether the platform currently considers the cursor visible.
        fn is_cursor_visible(&self) -> bool;
    }

    impl DisplayId {
        /// Create an identifier from a raw platform value.
        pub const fn new(id: u64) -> Self {
            Self(id)
        }
    }

    impl From<u64> for DisplayId {
        fn from(id: u64) -> Self {
            Self(id)
        }
    }

    impl From<DisplayId> for u64 {
        fn from(id: DisplayId) -> Self {
            id.0
        }
    }

    impl fmt::Display for DisplayId {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }
}

pub mod application;
pub mod clipboard;
pub mod context;
pub mod credentials;
pub mod entity;
pub mod notifications;
pub mod paths;
pub mod urls;

pub use application::*;
pub use clipboard::*;
pub use color::*;
pub use context::*;
pub use credentials::*;
pub use entity::*;
pub use geometry::*;
pub use input::*;
pub use notifications::*;
pub use paths::*;
pub use platform::*;
pub use urls::PlatformUrlSpi;
