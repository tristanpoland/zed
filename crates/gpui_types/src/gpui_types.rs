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
    use anyhow::{Context as _, anyhow};
    use schemars::JsonSchema;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
    use std::{
        borrow::Cow,
        cmp,
        fmt::{self, Debug, Display},
        hash::Hash,
        iter::Sum,
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub},
    };

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
    #[derive(Copy, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
    pub struct Pixels(pub f32);

    /// Device pixels.
    #[derive(
        Copy,
        Clone,
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
    pub struct DevicePixels(pub i32);

    /// Scaled logical pixels used by the paint protocol.
    #[derive(Copy, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
    pub struct ScaledPixels(pub f32);

    /// Root-relative units.
    #[derive(Copy, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
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

    /// Supplies the root-relative pixel size required by rem conversions.
    pub trait RemSizeProvider {
        /// Returns the size of one rem in logical pixels.
        fn rem_size(&self) -> Pixels;
    }

    impl Axis {
        /// Returns the opposite axis.
        pub fn invert(self) -> Self {
            match self {
                Self::Horizontal => Self::Vertical,
                Self::Vertical => Self::Horizontal,
            }
        }
    }

    impl Add for Pixels {
        type Output = Self;

        fn add(self, other: Self) -> Self {
            Self(self.0 + other.0)
        }
    }

    impl AddAssign for Pixels {
        fn add_assign(&mut self, other: Self) {
            self.0 += other.0;
        }
    }

    impl Sub for Pixels {
        type Output = Self;

        fn sub(self, other: Self) -> Self {
            Self(self.0 - other.0)
        }
    }

    impl std::ops::SubAssign for Pixels {
        fn sub_assign(&mut self, other: Self) {
            self.0 -= other.0;
        }
    }

    impl Neg for Pixels {
        type Output = Self;

        fn neg(self) -> Self {
            Self(-self.0)
        }
    }

    impl Mul<f32> for Pixels {
        type Output = Self;

        fn mul(self, factor: f32) -> Self {
            Self(self.0 * factor)
        }
    }

    impl Mul<usize> for Pixels {
        type Output = Self;

        fn mul(self, factor: usize) -> Self {
            self * factor as f32
        }
    }

    impl Mul<Pixels> for f32 {
        type Output = Pixels;

        fn mul(self, pixels: Pixels) -> Pixels {
            pixels * self
        }
    }

    impl Mul<Pixels> for usize {
        type Output = Pixels;

        fn mul(self, pixels: Pixels) -> Pixels {
            pixels * self
        }
    }

    impl Div<f32> for Pixels {
        type Output = Self;

        fn div(self, divisor: f32) -> Self {
            Self(self.0 / divisor)
        }
    }

    impl Div for Pixels {
        type Output = f32;

        fn div(self, divisor: Self) -> f32 {
            self.0 / divisor.0
        }
    }

    impl DivAssign for Pixels {
        fn div_assign(&mut self, divisor: Self) {
            self.0 /= divisor.0;
        }
    }

    impl MulAssign<f32> for Pixels {
        fn mul_assign(&mut self, factor: f32) {
            self.0 *= factor;
        }
    }

    impl Rem for Pixels {
        type Output = Self;

        fn rem(self, divisor: Self) -> Self {
            Self(self.0 % divisor.0)
        }
    }

    impl RemAssign for Pixels {
        fn rem_assign(&mut self, divisor: Self) {
            self.0 %= divisor.0;
        }
    }

    impl Display for Pixels {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{}px", self.0)
        }
    }

    impl Debug for Pixels {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            Display::fmt(self, formatter)
        }
    }

    impl Eq for Pixels {}

    impl cmp::PartialOrd for Pixels {
        fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for Pixels {
        fn cmp(&self, other: &Self) -> cmp::Ordering {
            self.0.total_cmp(&other.0)
        }
    }

    impl Hash for Pixels {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.0.to_bits().hash(state);
        }
    }

    impl Sum for Pixels {
        fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
            iter.fold(Self::ZERO, |total, value| total + value)
        }
    }

    impl<'a> Sum<&'a Pixels> for Pixels {
        fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
            iter.fold(Self::ZERO, |total, value| total + *value)
        }
    }

    impl TryFrom<&str> for Pixels {
        type Error = anyhow::Error;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            value
                .strip_suffix("px")
                .context("expected 'px' suffix")
                .and_then(|number| Ok(number.parse()?))
                .map(Self)
        }
    }

    impl Pixels {
        /// Represents zero pixels.
        pub const ZERO: Self = Self(0.0);
        /// The maximum representable pixel value.
        pub const MAX: Self = Self(f32::MAX);
        /// The minimum representable pixel value.
        pub const MIN: Self = Self(f32::MIN);

        /// Returns the raw value.
        pub fn as_f32(self) -> f32 {
            self.0
        }

        /// Rounds down to a whole pixel.
        pub fn floor(&self) -> Self {
            Self(self.0.floor())
        }

        /// Rounds to the nearest whole pixel.
        pub fn round(&self) -> Self {
            Self(self.0.round())
        }

        /// Rounds up to a whole pixel.
        pub fn ceil(&self) -> Self {
            Self(self.0.ceil())
        }

        /// Scales this value into scaled pixels.
        pub fn scale(&self, factor: f32) -> ScaledPixels {
            ScaledPixels(self.0 * factor)
        }

        /// Raises this value to a power.
        pub fn pow(&self, exponent: f32) -> Self {
            Self(self.0.powf(exponent))
        }

        /// Returns the absolute value.
        pub fn abs(&self) -> Self {
            Self(self.0.abs())
        }

        /// Returns the sign of this value.
        pub fn signum(&self) -> f32 {
            self.0.signum()
        }

        /// Converts this value to `f64`.
        pub fn to_f64(self) -> f64 {
            self.0 as f64
        }
    }

    impl From<f64> for Pixels {
        fn from(value: f64) -> Self {
            Self(value as f32)
        }
    }

    impl From<f32> for Pixels {
        fn from(value: f32) -> Self {
            Self(value)
        }
    }

    impl From<Pixels> for f32 {
        fn from(value: Pixels) -> Self {
            value.0
        }
    }

    impl From<&Pixels> for f32 {
        fn from(value: &Pixels) -> Self {
            value.0
        }
    }

    impl From<Pixels> for f64 {
        fn from(value: Pixels) -> Self {
            value.0 as f64
        }
    }

    impl From<Pixels> for u32 {
        fn from(value: Pixels) -> Self {
            value.0 as u32
        }
    }

    impl From<&Pixels> for u32 {
        fn from(value: &Pixels) -> Self {
            value.0 as u32
        }
    }

    impl From<u32> for Pixels {
        fn from(value: u32) -> Self {
            Self(value as f32)
        }
    }

    impl From<Pixels> for usize {
        fn from(value: Pixels) -> Self {
            value.0 as usize
        }
    }

    impl From<usize> for Pixels {
        fn from(value: usize) -> Self {
            Self(value as f32)
        }
    }

    impl DevicePixels {
        /// Converts this pixel count to a byte count.
        pub fn to_bytes(self, bytes_per_pixel: u8) -> u32 {
            self.0 as u32 * bytes_per_pixel as u32
        }
    }

    impl Add for DevicePixels {
        type Output = Self;

        fn add(self, other: Self) -> Self {
            Self(self.0 + other.0)
        }
    }

    impl AddAssign for DevicePixels {
        fn add_assign(&mut self, other: Self) {
            self.0 += other.0;
        }
    }

    impl Sub for DevicePixels {
        type Output = Self;

        fn sub(self, other: Self) -> Self {
            Self(self.0 - other.0)
        }
    }

    impl std::ops::SubAssign for DevicePixels {
        fn sub_assign(&mut self, other: Self) {
            self.0 -= other.0;
        }
    }

    impl fmt::Debug for DevicePixels {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{} px (device)", self.0)
        }
    }

    impl From<DevicePixels> for i32 {
        fn from(value: DevicePixels) -> Self {
            value.0
        }
    }

    impl From<i32> for DevicePixels {
        fn from(value: i32) -> Self {
            Self(value)
        }
    }

    impl From<u32> for DevicePixels {
        fn from(value: u32) -> Self {
            Self(value as i32)
        }
    }

    impl From<DevicePixels> for u32 {
        fn from(value: DevicePixels) -> Self {
            value.0 as u32
        }
    }

    impl From<DevicePixels> for u64 {
        fn from(value: DevicePixels) -> Self {
            value.0 as u64
        }
    }

    impl From<u64> for DevicePixels {
        fn from(value: u64) -> Self {
            Self(value as i32)
        }
    }

    impl From<DevicePixels> for usize {
        fn from(value: DevicePixels) -> Self {
            value.0 as usize
        }
    }

    impl From<usize> for DevicePixels {
        fn from(value: usize) -> Self {
            Self(value as i32)
        }
    }

    impl ScaledPixels {
        /// Returns the raw value.
        pub fn as_f32(self) -> f32 {
            self.0
        }

        /// Rounds down to a whole pixel.
        pub fn floor(&self) -> Self {
            Self(self.0.floor())
        }

        /// Rounds to the nearest whole pixel.
        pub fn round(&self) -> Self {
            Self(self.0.round())
        }

        /// Rounds up to a whole pixel.
        pub fn ceil(&self) -> Self {
            Self(self.0.ceil())
        }
    }

    impl Add for ScaledPixels {
        type Output = Self;

        fn add(self, other: Self) -> Self {
            Self(self.0 + other.0)
        }
    }

    impl AddAssign for ScaledPixels {
        fn add_assign(&mut self, other: Self) {
            self.0 += other.0;
        }
    }

    impl Sub for ScaledPixels {
        type Output = Self;

        fn sub(self, other: Self) -> Self {
            Self(self.0 - other.0)
        }
    }

    impl std::ops::SubAssign for ScaledPixels {
        fn sub_assign(&mut self, other: Self) {
            self.0 -= other.0;
        }
    }

    impl Eq for ScaledPixels {}

    impl cmp::PartialOrd for ScaledPixels {
        fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for ScaledPixels {
        fn cmp(&self, other: &Self) -> cmp::Ordering {
            self.0.total_cmp(&other.0)
        }
    }

    impl fmt::Debug for ScaledPixels {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{}px (scaled)", self.0)
        }
    }

    impl From<ScaledPixels> for DevicePixels {
        fn from(value: ScaledPixels) -> Self {
            Self(value.0.ceil() as i32)
        }
    }

    impl From<DevicePixels> for ScaledPixels {
        fn from(value: DevicePixels) -> Self {
            Self(value.0 as f32)
        }
    }

    impl From<ScaledPixels> for f64 {
        fn from(value: ScaledPixels) -> Self {
            value.0 as f64
        }
    }

    impl From<ScaledPixels> for u32 {
        fn from(value: ScaledPixels) -> Self {
            value.0 as u32
        }
    }

    impl From<f32> for ScaledPixels {
        fn from(value: f32) -> Self {
            Self(value)
        }
    }

    impl Div for ScaledPixels {
        type Output = f32;

        fn div(self, divisor: Self) -> f32 {
            self.0 / divisor.0
        }
    }

    impl DivAssign for ScaledPixels {
        fn div_assign(&mut self, divisor: Self) {
            self.0 /= divisor.0;
        }
    }

    impl Rem for ScaledPixels {
        type Output = Self;

        fn rem(self, divisor: Self) -> Self {
            Self(self.0 % divisor.0)
        }
    }

    impl RemAssign for ScaledPixels {
        fn rem_assign(&mut self, divisor: Self) {
            self.0 %= divisor.0;
        }
    }

    impl Mul<f32> for ScaledPixels {
        type Output = Self;

        fn mul(self, factor: f32) -> Self {
            Self(self.0 * factor)
        }
    }

    impl Mul<usize> for ScaledPixels {
        type Output = Self;

        fn mul(self, factor: usize) -> Self {
            self * factor as f32
        }
    }

    impl Mul<ScaledPixels> for f32 {
        type Output = ScaledPixels;

        fn mul(self, pixels: ScaledPixels) -> ScaledPixels {
            pixels * self
        }
    }

    impl Mul<ScaledPixels> for usize {
        type Output = ScaledPixels;

        fn mul(self, pixels: ScaledPixels) -> ScaledPixels {
            pixels * self
        }
    }

    impl MulAssign<f32> for ScaledPixels {
        fn mul_assign(&mut self, factor: f32) {
            self.0 *= factor;
        }
    }

    impl Rems {
        /// A length of zero.
        pub const ZERO: Self = Self(0.0);

        /// Converts this value to pixels using the given root-relative size.
        pub fn to_pixels(self, rem_size: Pixels) -> Pixels {
            self * rem_size
        }

        /// Converts pixels to rems using a runtime-provided root-relative size.
        pub fn from_pixels<T: RemSizeProvider>(length: Pixels, provider: &T) -> Self {
            Self(length / provider.rem_size())
        }
    }

    impl Mul<Pixels> for Rems {
        type Output = Pixels;

        fn mul(self, pixels: Pixels) -> Pixels {
            Pixels(self.0 * pixels.0)
        }
    }

    impl AddAssign for Rems {
        fn add_assign(&mut self, other: Self) {
            self.0 += other.0;
        }
    }

    impl Add for Rems {
        type Output = Self;

        fn add(self, other: Self) -> Self {
            Self(self.0 + other.0)
        }
    }

    impl Sub for Rems {
        type Output = Self;

        fn sub(self, other: Self) -> Self {
            Self(self.0 - other.0)
        }
    }

    impl Mul for Rems {
        type Output = Self;

        fn mul(self, factor: Self) -> Self {
            Self(self.0 * factor.0)
        }
    }

    impl Mul<f32> for Rems {
        type Output = Self;

        fn mul(self, factor: f32) -> Self {
            Self(self.0 * factor)
        }
    }

    impl Div for Rems {
        type Output = Self;

        fn div(self, divisor: Self) -> Self {
            Self(self.0 / divisor.0)
        }
    }

    impl Neg for Rems {
        type Output = Self;

        fn neg(self) -> Self {
            Self(-self.0)
        }
    }

    impl Display for Rems {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{}rem", self.0)
        }
    }

    impl Debug for Rems {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            Display::fmt(self, formatter)
        }
    }

    impl TryFrom<&str> for Rems {
        type Error = anyhow::Error;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            value
                .strip_suffix("rem")
                .context("expected 'rem' suffix")
                .and_then(|number| Ok(number.parse()?))
                .map(Self)
        }
    }

    /// An absolute length in pixels or rems.
    #[derive(Clone, Copy, PartialEq)]
    pub enum AbsoluteLength {
        /// A length in pixels.
        Pixels(Pixels),
        /// A length in rems.
        Rems(Rems),
    }

    impl AbsoluteLength {
        /// Returns whether this length is zero.
        pub fn is_zero(&self) -> bool {
            match self {
                Self::Pixels(pixels) => pixels.0 == 0.0,
                Self::Rems(rems) => rems.0 == 0.0,
            }
        }

        /// Converts this length to pixels.
        pub fn to_pixels(self, rem_size: Pixels) -> Pixels {
            match self {
                Self::Pixels(pixels) => pixels,
                Self::Rems(rems) => rems.to_pixels(rem_size),
            }
        }

        /// Converts this length to rems.
        pub fn to_rems(self, rem_size: Pixels) -> Rems {
            match self {
                Self::Pixels(pixels) => Rems(pixels.0 / rem_size.0),
                Self::Rems(rems) => rems,
            }
        }
    }

    impl Neg for AbsoluteLength {
        type Output = Self;

        fn neg(self) -> Self {
            match self {
                Self::Pixels(pixels) => Self::Pixels(-pixels),
                Self::Rems(rems) => Self::Rems(-rems),
            }
        }
    }

    impl From<Pixels> for AbsoluteLength {
        fn from(value: Pixels) -> Self {
            Self::Pixels(value)
        }
    }

    impl From<Rems> for AbsoluteLength {
        fn from(value: Rems) -> Self {
            Self::Rems(value)
        }
    }

    impl Default for AbsoluteLength {
        fn default() -> Self {
            px(0.0).into()
        }
    }

    impl Display for AbsoluteLength {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Pixels(pixels) => write!(formatter, "{pixels}"),
                Self::Rems(rems) => write!(formatter, "{rems}"),
            }
        }
    }

    impl Debug for AbsoluteLength {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            Display::fmt(self, formatter)
        }
    }

    impl TryFrom<&str> for AbsoluteLength {
        type Error = anyhow::Error;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            if let Ok(pixels) = value.try_into() {
                Ok(Self::Pixels(pixels))
            } else if let Ok(rems) = value.try_into() {
                Ok(Self::Rems(rems))
            } else {
                Err(anyhow!(
                    "invalid AbsoluteLength '{value}', expected number with 'px' or 'rem' suffix"
                ))
            }
        }
    }

    impl JsonSchema for AbsoluteLength {
        fn schema_name() -> Cow<'static, str> {
            "AbsoluteLength".into()
        }

        fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
            schemars::json_schema!({
                "type": "string",
                "pattern": r"^-?\d+(\.\d+)?(px|rem)$"
            })
        }
    }

    impl<'de> Deserialize<'de> for AbsoluteLength {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct StringVisitor;

            impl de::Visitor<'_> for StringVisitor {
                type Value = AbsoluteLength;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("number with 'px' or 'rem' suffix")
                }

                fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                    AbsoluteLength::try_from(value).map_err(E::custom)
                }
            }

            deserializer.deserialize_str(StringVisitor)
        }
    }

    impl Serialize for AbsoluteLength {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(&format!("{self}"))
        }
    }

    /// A non-auto length in pixels, rems, or a parent-relative fraction.
    #[derive(Clone, Copy, PartialEq)]
    pub enum DefiniteLength {
        /// An absolute length.
        Absolute(AbsoluteLength),
        /// A fraction of the parent's size.
        Fraction(f32),
    }

    impl DefiniteLength {
        /// Converts this length to pixels.
        pub fn to_pixels(self, base_size: AbsoluteLength, rem_size: Pixels) -> Pixels {
            match self {
                Self::Absolute(length) => length.to_pixels(rem_size),
                Self::Fraction(fraction) => match base_size {
                    AbsoluteLength::Pixels(pixels) => pixels * fraction,
                    AbsoluteLength::Rems(rems) => rems * rem_size * fraction,
                },
            }
        }

        /// Returns whether this length is zero.
        pub fn is_zero(&self) -> bool {
            match self {
                Self::Absolute(length) => length.is_zero(),
                Self::Fraction(fraction) => *fraction == 0.0,
            }
        }
    }

    impl Neg for DefiniteLength {
        type Output = Self;

        fn neg(self) -> Self {
            match self {
                Self::Absolute(length) => Self::Absolute(-length),
                Self::Fraction(fraction) => Self::Fraction(-fraction),
            }
        }
    }

    impl Debug for DefiniteLength {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            Display::fmt(self, formatter)
        }
    }

    impl Display for DefiniteLength {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Absolute(length) => write!(formatter, "{length}"),
                Self::Fraction(fraction) => write!(formatter, "{}%", (fraction * 100.0) as i32),
            }
        }
    }

    impl TryFrom<&str> for DefiniteLength {
        type Error = anyhow::Error;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            if let Some(percentage) = value.strip_suffix('%') {
                let fraction = percentage.parse::<f32>().with_context(|| {
                    format!("invalid DefiniteLength '{value}', expected number with 'px', 'rem', or '%' suffix")
                })?;
                Ok(Self::Fraction(fraction / 100.0))
            } else if let Ok(absolute_length) = value.try_into() {
                Ok(Self::Absolute(absolute_length))
            } else {
                Err(anyhow!(
                    "invalid DefiniteLength '{value}', expected number with 'px', 'rem', or '%' suffix"
                ))
            }
        }
    }

    impl JsonSchema for DefiniteLength {
        fn schema_name() -> Cow<'static, str> {
            "DefiniteLength".into()
        }

        fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
            schemars::json_schema!({
                "type": "string",
                "pattern": r"^-?\d+(\.\d+)?(px|rem|%)$"
            })
        }
    }

    impl<'de> Deserialize<'de> for DefiniteLength {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct StringVisitor;

            impl de::Visitor<'_> for StringVisitor {
                type Value = DefiniteLength;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("number with 'px', 'rem', or '%' suffix")
                }

                fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                    DefiniteLength::try_from(value).map_err(E::custom)
                }
            }

            deserializer.deserialize_str(StringVisitor)
        }
    }

    impl Serialize for DefiniteLength {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(&format!("{self}"))
        }
    }

    impl From<Pixels> for DefiniteLength {
        fn from(value: Pixels) -> Self {
            Self::Absolute(value.into())
        }
    }

    impl From<Rems> for DefiniteLength {
        fn from(value: Rems) -> Self {
            Self::Absolute(value.into())
        }
    }

    impl From<AbsoluteLength> for DefiniteLength {
        fn from(value: AbsoluteLength) -> Self {
            Self::Absolute(value)
        }
    }

    impl Default for DefiniteLength {
        fn default() -> Self {
            Self::Absolute(AbsoluteLength::default())
        }
    }

    /// A length in pixels, rems, a parent-relative fraction, or auto.
    #[derive(Clone, Copy, PartialEq)]
    pub enum Length {
        /// A definite length.
        Definite(DefiniteLength),
        /// An automatic length.
        Auto,
    }

    impl Length {
        /// Returns whether this length is zero.
        pub fn is_zero(&self) -> bool {
            match self {
                Self::Definite(length) => length.is_zero(),
                Self::Auto => false,
            }
        }
    }

    impl Neg for Length {
        type Output = Self;

        fn neg(self) -> Self {
            match self {
                Self::Definite(length) => Self::Definite(-length),
                Self::Auto => Self::Auto,
            }
        }
    }

    impl Debug for Length {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            Display::fmt(self, formatter)
        }
    }

    impl Display for Length {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Definite(length) => write!(formatter, "{length}"),
                Self::Auto => formatter.write_str("auto"),
            }
        }
    }

    impl TryFrom<&str> for Length {
        type Error = anyhow::Error;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            if value == "auto" {
                Ok(Self::Auto)
            } else if let Ok(definite_length) = value.try_into() {
                Ok(Self::Definite(definite_length))
            } else {
                Err(anyhow!(
                    "invalid Length '{value}', expected 'auto' or number with 'px', 'rem', or '%' suffix"
                ))
            }
        }
    }

    impl JsonSchema for Length {
        fn schema_name() -> Cow<'static, str> {
            "Length".into()
        }

        fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
            schemars::json_schema!({
                "type": "string",
                "pattern": r"^(auto|-?\d+(\.\d+)?(px|rem|%))$"
            })
        }
    }

    impl<'de> Deserialize<'de> for Length {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct StringVisitor;

            impl de::Visitor<'_> for StringVisitor {
                type Value = Length;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("'auto' or number with 'px', 'rem', or '%' suffix")
                }

                fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                    Length::try_from(value).map_err(E::custom)
                }
            }

            deserializer.deserialize_str(StringVisitor)
        }
    }

    impl Serialize for Length {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(&format!("{self}"))
        }
    }

    impl Default for Length {
        fn default() -> Self {
            Self::Definite(DefiniteLength::default())
        }
    }

    impl From<Pixels> for Length {
        fn from(value: Pixels) -> Self {
            Self::Definite(value.into())
        }
    }

    impl From<Rems> for Length {
        fn from(value: Rems) -> Self {
            Self::Definite(value.into())
        }
    }

    impl From<DefiniteLength> for Length {
        fn from(value: DefiniteLength) -> Self {
            Self::Definite(value)
        }
    }

    impl From<AbsoluteLength> for Length {
        fn from(value: AbsoluteLength) -> Self {
            Self::Definite(value.into())
        }
    }

    impl From<()> for Length {
        fn from(_: ()) -> Self {
            Self::default()
        }
    }

    /// Constructs a relative parent-size fraction.
    pub const fn relative(fraction: f32) -> DefiniteLength {
        DefiniteLength::Fraction(fraction)
    }

    /// Returns the golden ratio as a relative length.
    pub const fn phi() -> DefiniteLength {
        relative(1.618_034)
    }

    /// Constructs an automatic length.
    pub const fn auto() -> Length {
        Length::Auto
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
        /// Returns whether any modifier key is pressed.
        pub fn modified(&self) -> bool {
            self.control || self.alt || self.shift || self.platform || self.function
        }

        /// Whether the semantically 'secondary' modifier key is pressed.
        pub fn secondary(&self) -> bool {
            #[cfg(target_os = "macos")]
            {
                self.platform
            }

            #[cfg(not(target_os = "macos"))]
            {
                self.control
            }
        }

        /// Returns how many modifier keys are pressed.
        pub fn number_of_modifiers(&self) -> u8 {
            self.control as u8
                + self.alt as u8
                + self.shift as u8
                + self.platform as u8
                + self.function as u8
        }

        /// Returns [`Modifiers`] with no modifiers.
        pub fn none() -> Self {
            Default::default()
        }

        /// Returns [`Modifiers`] with just the command key.
        pub fn command() -> Self {
            Self {
                platform: true,
                ..Default::default()
            }
        }

        /// Returns [`Modifiers`] with just the secondary key pressed.
        pub fn secondary_key() -> Self {
            #[cfg(target_os = "macos")]
            {
                Self {
                    platform: true,
                    ..Default::default()
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                Self {
                    control: true,
                    ..Default::default()
                }
            }
        }

        /// Returns [`Modifiers`] with just the windows key.
        pub fn windows() -> Self {
            Self {
                platform: true,
                ..Default::default()
            }
        }

        /// Returns [`Modifiers`] with just the super key.
        pub fn super_key() -> Self {
            Self {
                platform: true,
                ..Default::default()
            }
        }

        /// Returns [`Modifiers`] with just control.
        pub fn control() -> Self {
            Self {
                control: true,
                ..Default::default()
            }
        }

        /// Returns [`Modifiers`] with just alt.
        pub fn alt() -> Self {
            Self {
                alt: true,
                ..Default::default()
            }
        }

        /// Returns [`Modifiers`] with just shift.
        pub fn shift() -> Self {
            Self {
                shift: true,
                ..Default::default()
            }
        }

        /// Returns [`Modifiers`] with just function.
        pub fn function() -> Self {
            Self {
                function: true,
                ..Default::default()
            }
        }

        /// Returns [`Modifiers`] with command + shift.
        pub fn command_shift() -> Self {
            Self {
                shift: true,
                platform: true,
                ..Default::default()
            }
        }

        /// Returns [`Modifiers`] with control + shift.
        pub fn control_shift() -> Self {
            Self {
                shift: true,
                control: true,
                ..Default::default()
            }
        }

        /// Checks if this [`Modifiers`] is a subset of another [`Modifiers`].
        pub fn is_subset_of(&self, other: &Self) -> bool {
            (*other & *self) == *self
        }
    }

    impl std::ops::BitOr for Modifiers {
        type Output = Self;

        fn bitor(mut self, other: Self) -> Self::Output {
            self |= other;
            self
        }
    }

    impl std::ops::BitOrAssign for Modifiers {
        fn bitor_assign(&mut self, other: Self) {
            self.control |= other.control;
            self.alt |= other.alt;
            self.shift |= other.shift;
            self.platform |= other.platform;
            self.function |= other.function;
        }
    }

    impl std::ops::BitXor for Modifiers {
        type Output = Self;

        fn bitxor(mut self, rhs: Self) -> Self::Output {
            self ^= rhs;
            self
        }
    }

    impl std::ops::BitXorAssign for Modifiers {
        fn bitxor_assign(&mut self, other: Self) {
            self.control ^= other.control;
            self.alt ^= other.alt;
            self.shift ^= other.shift;
            self.platform ^= other.platform;
            self.function ^= other.function;
        }
    }

    impl std::ops::BitAnd for Modifiers {
        type Output = Self;

        fn bitand(mut self, rhs: Self) -> Self::Output {
            self &= rhs;
            self
        }
    }

    impl std::ops::BitAndAssign for Modifiers {
        fn bitand_assign(&mut self, other: Self) {
            self.control &= other.control;
            self.alt &= other.alt;
            self.shift &= other.shift;
            self.platform &= other.platform;
            self.function &= other.function;
        }
    }

    /// The state of the caps lock key.
    #[derive(
        Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
    )]
    pub struct Capslock {
        /// The capslock key is on.
        #[serde(default)]
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
    pub use crate::application::{AppLifecyclePhase, PlatformApplicationSpi, PlatformServicesSpi};
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

pub mod accessibility;
pub mod application;
pub mod clipboard;
pub mod context;
pub mod credentials;
pub mod entity;
pub mod input_method;
pub mod keyboard;
pub mod keystroke;
pub mod notifications;
pub mod paths;
pub mod urls;
pub mod window;

pub use accessibility::{
    A11yCallbacks, AccessibleAction, Action, ActionData, ActionRequest, Node, NodeId,
    NodeIdContent, Orientation, PlatformAccessibilitySpi, Rect, Role, TextPosition, TextSelection,
    Toggled, Tree, TreeId, TreeUpdate,
};
pub use application::*;
pub use clipboard::*;
pub use color::*;
pub use context::*;
pub use credentials::*;
pub use entity::*;
pub use geometry::*;
pub use input::*;
pub use input_method::*;
pub use keyboard::*;
pub use keystroke::*;
pub use notifications::*;
pub use paths::*;
pub use platform::*;
pub use urls::PlatformUrlSpi;
pub use window::*;
