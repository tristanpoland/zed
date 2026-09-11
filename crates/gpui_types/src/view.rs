//! Backend-neutral view identities and element metadata.

use crate::EntityId;
use anyhow::Result;
use std::{any::TypeId, fmt};

/// The backend capability used by shared view handles and element metadata.
pub trait ViewBackend: 'static {
    /// The backend's type-erased strong entity handle.
    type AnyEntity: Clone + PartialEq;
    /// The backend's type-erased weak entity handle.
    type AnyWeakEntity: Clone + PartialEq;
    /// The backend's typed strong entity handle family.
    type Entity<T>;
    /// The backend's typed weak entity handle family.
    type WeakEntity<T>;
    /// The backend window passed to view render callbacks.
    type Window;
    /// The backend application context passed to view render callbacks.
    type App;
    /// The backend element produced by rendering a view.
    type Element;
    /// The backend style used to configure a cached view.
    type CacheStyle;

    /// Converts a typed entity into the backend's type-erased handle.
    fn into_any<T: 'static>(entity: Self::Entity<T>) -> Self::AnyEntity;

    /// Converts a typed weak entity into the backend's type-erased handle.
    fn into_any_weak<T: 'static>(entity: Self::WeakEntity<T>) -> Self::AnyWeakEntity;

    /// Returns the entity ID of a strong handle.
    fn entity_id(entity: &Self::AnyEntity) -> EntityId;

    /// Returns the entity ID of a weak handle.
    fn weak_entity_id(entity: &Self::AnyWeakEntity) -> EntityId;

    /// Returns the type ID of a strong handle.
    fn entity_type(entity: &Self::AnyEntity) -> TypeId;

    /// Downgrades a strong entity handle.
    fn downgrade(entity: &Self::AnyEntity) -> Self::AnyWeakEntity;

    /// Downcasts a strong entity handle.
    fn downcast<T: 'static>(entity: Self::AnyEntity) -> Result<Self::Entity<T>, Self::AnyEntity>;

    /// Upgrades a weak entity handle.
    fn upgrade(entity: &Self::AnyWeakEntity) -> Option<Self::AnyEntity>;
}

/// A dynamically typed view handle whose rendering callback is supplied by a backend.
pub struct AnyView<B: ViewBackend> {
    entity: B::AnyEntity,
    render: fn(&AnyView<B>, &mut B::Window, &mut B::App) -> B::Element,
}

impl<B: ViewBackend> Clone for AnyView<B> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity.clone(),
            render: self.render,
        }
    }
}

impl<B: ViewBackend> fmt::Debug for AnyView<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnyView")
            .field("entity_id", &B::entity_id(&self.entity))
            .finish_non_exhaustive()
    }
}

impl<B: ViewBackend> AnyView<B> {
    /// Creates a view handle from its entity and backend rendering callback.
    #[doc(hidden)]
    pub fn from_parts(
        entity: B::AnyEntity,
        render: fn(&AnyView<B>, &mut B::Window, &mut B::App) -> B::Element,
    ) -> Self {
        Self { entity, render }
    }

    /// Embeds this view as a cached view element.
    pub fn cached(self, style: B::CacheStyle) -> ViewElement<B, Self> {
        let entity_id = self.entity_id();
        ViewElement::new_with_entity(self, Some(entity_id)).cached(style)
    }

    /// Converts this to a weak handle.
    pub fn downgrade(&self) -> AnyWeakView<B> {
        AnyWeakView {
            entity: B::downgrade(&self.entity),
            render: self.render,
        }
    }

    /// Converts this to an entity of a specific type.
    pub fn downcast<T: 'static>(self) -> Result<B::Entity<T>, Self> {
        match B::downcast(self.entity) {
            Ok(entity) => Ok(entity),
            Err(entity) => Err(Self {
                entity,
                render: self.render,
            }),
        }
    }

    /// Gets the type ID of the underlying view.
    pub fn entity_type(&self) -> TypeId {
        B::entity_type(&self.entity)
    }

    /// Gets the entity ID of this view.
    pub fn entity_id(&self) -> EntityId {
        B::entity_id(&self.entity)
    }

    /// Renders this view through its backend callback.
    #[doc(hidden)]
    pub fn render_with(&self, window: &mut B::Window, app: &mut B::App) -> B::Element {
        (self.render)(self, window, app)
    }
}

impl<B: ViewBackend> PartialEq for AnyView<B> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
    }
}

impl<B: ViewBackend> Eq for AnyView<B> {}

/// A weak, dynamically typed view handle.
pub struct AnyWeakView<B: ViewBackend> {
    entity: B::AnyWeakEntity,
    render: fn(&AnyView<B>, &mut B::Window, &mut B::App) -> B::Element,
}

impl<B: ViewBackend> AnyWeakView<B> {
    /// Creates a weak view handle from its entity and backend rendering callback.
    #[doc(hidden)]
    pub fn from_parts(
        entity: B::AnyWeakEntity,
        render: fn(&AnyView<B>, &mut B::Window, &mut B::App) -> B::Element,
    ) -> Self {
        Self { entity, render }
    }

    /// Upgrades to a strong view handle if the view is still alive.
    pub fn upgrade(&self) -> Option<AnyView<B>> {
        let entity = B::upgrade(&self.entity)?;
        Some(AnyView {
            entity,
            render: self.render,
        })
    }
}

impl<B: ViewBackend> PartialEq for AnyWeakView<B> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
    }
}

impl<B: ViewBackend> fmt::Debug for AnyWeakView<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnyWeakView")
            .field("entity_id", &B::weak_entity_id(&self.entity))
            .finish_non_exhaustive()
    }
}

/// Backend-neutral metadata and ownership for a rendered view element.
#[doc(hidden)]
pub struct ViewElement<B: ViewBackend, V: 'static> {
    view: Option<V>,
    entity_id: Option<EntityId>,
    cached_style: Option<B::CacheStyle>,
    #[cfg(debug_assertions)]
    source: &'static core::panic::Location<'static>,
}

impl<B: ViewBackend, V: 'static> ViewElement<B, V> {
    /// Wraps a view as an element.
    #[track_caller]
    pub fn new(view: V) -> Self {
        Self::new_with_entity(view, None)
    }

    /// Wraps a view as an element with an explicit reactive identity.
    #[doc(hidden)]
    #[track_caller]
    pub fn new_with_entity(view: V, entity_id: Option<EntityId>) -> Self {
        Self {
            entity_id,
            cached_style: None,
            view: Some(view),
            #[cfg(debug_assertions)]
            source: core::panic::Location::caller(),
        }
    }

    /// Enables caching of this view's rendered subtree.
    #[doc(hidden)]
    pub fn cached(mut self, style: B::CacheStyle) -> Self {
        self.cached_style = Some(style);
        self
    }

    /// Takes the view for backend rendering.
    #[doc(hidden)]
    pub fn take_view(&mut self) -> Option<V> {
        self.view.take()
    }

    /// Borrows the view while it is still owned by this element.
    #[doc(hidden)]
    pub fn view(&self) -> Option<&V> {
        self.view.as_ref()
    }

    /// Stores the reactive identity discovered by the backend.
    #[doc(hidden)]
    pub fn set_view_entity_id(&mut self, entity_id: Option<EntityId>) {
        self.entity_id = entity_id;
    }

    /// Returns this view's identity, if it has one.
    #[doc(hidden)]
    pub fn view_entity_id(&self) -> Option<EntityId> {
        self.entity_id
    }

    /// Returns the cache style supplied for this element, if any.
    #[doc(hidden)]
    pub fn cached_style(&self) -> Option<&B::CacheStyle> {
        self.cached_style.as_ref()
    }

    /// Returns whether this element has cached rendering enabled.
    #[doc(hidden)]
    pub fn is_cached(&self) -> bool {
        self.cached_style.is_some()
    }

    /// Returns the source location where this element was constructed.
    #[doc(hidden)]
    pub fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        #[cfg(debug_assertions)]
        return Some(self.source);

        #[cfg(not(debug_assertions))]
        None
    }
}
