//! Backend-neutral element path and inspector metadata.

use gpui_shared_string::SharedString;
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// An identifier for an element.
///
/// The focus identifier is supplied by the backend because focus handles are
/// owned by the window implementation. All other identity forms are shared
/// between GPUI API facades and backends.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ElementId<F> {
    /// The ID of a view element.
    View(crate::EntityId),
    /// An integer ID.
    Integer(u64),
    /// A string-based ID.
    Name(SharedString),
    /// A UUID.
    Uuid(uuid::Uuid),
    /// An ID associated with a focus handle.
    FocusHandle(F),
    /// A combination of a name and an integer.
    NamedInteger(SharedString, u64),
    /// A path.
    Path(Arc<std::path::Path>),
    /// A code location.
    CodeLocation(core::panic::Location<'static>),
    /// A labeled child of an element.
    NamedChild(Arc<ElementId<F>>, SharedString),
    /// A byte-array ID (used for text anchors).
    OpaqueId([u8; 20]),
}

impl<F> ElementId<F> {
    /// Constructs an `ElementId::NamedInteger` from a name and `usize`.
    pub fn named_usize(name: impl Into<SharedString>, integer: usize) -> Self {
        Self::NamedInteger(name.into(), integer as u64)
    }
}

impl<F> Display for ElementId<F> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::View(entity_id) => write!(formatter, "view-{entity_id}")?,
            Self::Integer(integer) => write!(formatter, "{integer}")?,
            Self::Name(name) => write!(formatter, "{name}")?,
            Self::FocusHandle(_) => write!(formatter, "FocusHandle")?,
            Self::NamedInteger(name, integer) => write!(formatter, "{name}-{integer}")?,
            Self::Uuid(uuid) => write!(formatter, "{uuid}")?,
            Self::Path(path) => write!(formatter, "{}", path.display())?,
            Self::CodeLocation(location) => write!(formatter, "{location}")?,
            Self::NamedChild(id, name) => write!(formatter, "{id}-{name}")?,
            Self::OpaqueId(opaque_id) => write!(formatter, "{opaque_id:x?}")?,
        }
        Ok(())
    }
}

impl<F> TryInto<SharedString> for ElementId<F> {
    type Error = anyhow::Error;

    fn try_into(self) -> anyhow::Result<SharedString> {
        if let Self::Name(name) = self {
            Ok(name)
        } else {
            anyhow::bail!("element id is not string")
        }
    }
}

impl<F> From<usize> for ElementId<F> {
    fn from(id: usize) -> Self {
        Self::Integer(id as u64)
    }
}

impl<F> From<i32> for ElementId<F> {
    fn from(id: i32) -> Self {
        Self::Integer(id as u64)
    }
}

impl<F> From<SharedString> for ElementId<F> {
    fn from(name: SharedString) -> Self {
        Self::Name(name)
    }
}

impl<F> From<String> for ElementId<F> {
    fn from(name: String) -> Self {
        Self::Name(name.into())
    }
}

impl<F> From<Arc<str>> for ElementId<F> {
    fn from(name: Arc<str>) -> Self {
        Self::Name(name.into())
    }
}

impl<F> From<Arc<std::path::Path>> for ElementId<F> {
    fn from(path: Arc<std::path::Path>) -> Self {
        Self::Path(path)
    }
}

impl<F> From<&'static str> for ElementId<F> {
    fn from(name: &'static str) -> Self {
        Self::Name(SharedString::new_static(name))
    }
}

impl<F> From<(&'static str, crate::EntityId)> for ElementId<F> {
    fn from((name, id): (&'static str, crate::EntityId)) -> Self {
        Self::NamedInteger(SharedString::new_static(name), id.as_u64())
    }
}

impl<F> From<(&'static str, usize)> for ElementId<F> {
    fn from((name, id): (&'static str, usize)) -> Self {
        Self::NamedInteger(SharedString::new_static(name), id as u64)
    }
}

impl<F> From<(SharedString, usize)> for ElementId<F> {
    fn from((name, id): (SharedString, usize)) -> Self {
        Self::NamedInteger(name, id as u64)
    }
}

impl<F> From<(&'static str, u64)> for ElementId<F> {
    fn from((name, id): (&'static str, u64)) -> Self {
        Self::NamedInteger(SharedString::new_static(name), id)
    }
}

impl<F> From<uuid::Uuid> for ElementId<F> {
    fn from(value: uuid::Uuid) -> Self {
        Self::Uuid(value)
    }
}

impl<F> From<(&'static str, u32)> for ElementId<F> {
    fn from((name, id): (&'static str, u32)) -> Self {
        Self::NamedInteger(SharedString::new_static(name), u64::from(id))
    }
}

impl<F, T: Into<SharedString>> From<(ElementId<F>, T)> for ElementId<F> {
    fn from((id, name): (ElementId<F>, T)) -> Self {
        Self::NamedChild(Arc::new(id), name.into())
    }
}

impl<F> From<&'static core::panic::Location<'static>> for ElementId<F> {
    fn from(location: &'static core::panic::Location<'static>) -> Self {
        Self::CodeLocation(*location)
    }
}

impl<F> From<[u8; 20]> for ElementId<F> {
    fn from(opaque_id: [u8; 20]) -> Self {
        Self::OpaqueId(opaque_id)
    }
}

#[cfg(test)]
mod tests {
    use super::ElementId;

    #[test]
    fn element_id_constructors_preserve_display_and_nesting() {
        let parent = ElementId::<()>::from("parent");
        let child = ElementId::from((parent, "child"));

        assert_eq!(child.to_string(), "parent-child");
        assert_eq!(ElementId::<()>::from(42usize).to_string(), "42");
    }
}

/// A globally unique element path used to track state across frames.
pub struct GlobalElementId<I>(Arc<[I]>);

impl<I> GlobalElementId<I> {
    /// Creates an element path from its component IDs.
    pub fn from_ids(ids: Arc<[I]>) -> Self {
        Self(ids)
    }

    /// Returns the element path represented by this ID.
    pub fn as_slice(&self) -> &[I] {
        &self.0
    }
}

impl<I> From<Arc<[I]>> for GlobalElementId<I> {
    fn from(ids: Arc<[I]>) -> Self {
        Self::from_ids(ids)
    }
}

impl<I> Clone for GlobalElementId<I> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<I> Default for GlobalElementId<I> {
    fn default() -> Self {
        Self(Arc::default())
    }
}

impl<I: std::fmt::Debug> std::fmt::Debug for GlobalElementId<I> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<I: Eq> Eq for GlobalElementId<I> {}

impl<I: PartialEq> PartialEq for GlobalElementId<I> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<I: std::hash::Hash> std::hash::Hash for GlobalElementId<I> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<I> Deref for GlobalElementId<I> {
    type Target = [I];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<I: Clone> DerefMut for GlobalElementId<I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl<I: Display> Display for GlobalElementId<I> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, element_id) in self.0.iter().enumerate() {
            if index > 0 {
                write!(formatter, ".")?;
            }
            write!(formatter, "{}", element_id)?;
        }
        Ok(())
    }
}

impl<I: std::hash::Hash> GlobalElementId<I> {
    /// Returns the AccessKit node ID corresponding to this element path.
    pub fn accesskit_node_id(&self) -> accesskit::NodeId {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::default();
        self.hash(&mut hasher);
        accesskit::NodeId(hasher.finish())
    }
}

/// A unique identifier for an element that can be inspected.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct InspectorElementId<I> {
    /// Stable part of the ID.
    #[cfg(any(feature = "inspector", debug_assertions))]
    pub path: std::rc::Rc<InspectorElementPath<I>>,
    /// Disambiguates elements that have the same path.
    #[cfg(any(feature = "inspector", debug_assertions))]
    pub instance_id: usize,
}

impl<I: Clone> Into<InspectorElementId<I>> for &InspectorElementId<I> {
    fn into(self) -> InspectorElementId<I> {
        self.clone()
    }
}

/// A global element ID qualified by the source location where an element was constructed.
#[cfg(any(feature = "inspector", debug_assertions))]
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct InspectorElementPath<I> {
    /// The path to the nearest ancestor element that has an element ID.
    pub global_id: GlobalElementId<I>,
    /// Source location where this element was constructed.
    pub source_location: &'static std::panic::Location<'static>,
}

#[cfg(any(feature = "inspector", debug_assertions))]
impl<I: Clone> Clone for InspectorElementPath<I> {
    fn clone(&self) -> Self {
        Self {
            global_id: self.global_id.clone(),
            source_location: self.source_location,
        }
    }
}

#[cfg(any(feature = "inspector", debug_assertions))]
impl<I: Clone> Into<InspectorElementPath<I>> for &InspectorElementPath<I> {
    fn into(self) -> InspectorElementPath<I> {
        self.clone()
    }
}
