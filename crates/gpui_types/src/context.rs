use super::entity::{Entity, EntityHandleRuntime, WeakEntity};
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// The backend capability used by the shared application and entity context
/// types.
pub trait AppContextRuntime: 'static {
    /// Returns the handle-lifetime capability for this application.
    fn entity_runtime(&self) -> Rc<dyn EntityHandleRuntime>;
}

/// A backend-neutral application context.
pub struct App {
    runtime: Rc<dyn AppContextRuntime>,
}

impl App {
    /// Builds a shared application context from a backend capability.
    #[doc(hidden)]
    pub fn from_runtime(runtime: Rc<dyn AppContextRuntime>) -> Self {
        Self { runtime }
    }

    /// Returns whether an entity is currently live in the application.
    pub fn entity_exists(&self, entity_id: super::EntityId) -> bool {
        self.runtime.entity_runtime().is_upgradable(entity_id)
    }

    /// Creates an entity context for a weak entity identity.
    #[doc(hidden)]
    pub fn new_context<T>(&mut self, entity_state: WeakEntity<T>) -> Context<'_, T> {
        Context {
            app: self,
            entity_state,
        }
    }
}

/// The app context specialized for the state associated with one entity.
pub struct Context<'a, T> {
    app: &'a mut App,
    entity_state: WeakEntity<T>,
}

impl<T> Deref for Context<'_, T> {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        self.app
    }
}

impl<T> DerefMut for Context<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.app
    }
}

impl<T> Context<'_, T> {
    /// Builds a context from an application and the weak identity of its state.
    #[doc(hidden)]
    pub fn new_context<'a>(app: &'a mut App, entity_state: WeakEntity<T>) -> Context<'a, T> {
        Context { app, entity_state }
    }

    /// Returns the entity id backing this context.
    pub fn entity_id(&self) -> super::EntityId {
        self.entity_state.entity_id()
    }

    /// Attempts to obtain a strong handle for the entity backing this context.
    pub fn entity(&self) -> Option<Entity<T>>
    where
        T: 'static,
    {
        self.entity_state.upgrade()
    }

    /// Returns a weak handle for the entity backing this context.
    pub fn weak_entity(&self) -> WeakEntity<T> {
        WeakEntity::clone(&self.entity_state)
    }
}
