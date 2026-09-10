//! Backend-neutral data types used by GPUI APIs and implementations.

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

pub use color::*;
pub use geometry::*;
pub use input::*;
pub use platform::*;
