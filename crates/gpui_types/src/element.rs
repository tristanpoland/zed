//! Backend-neutral element path and inspector metadata.

use std::{fmt::Display, ops::{Deref, DerefMut}, sync::Arc};

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
