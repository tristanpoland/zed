use super::{
    EntityHandle, EntityId, Global, SubscriptionHandle, TaskHandle,
    entity::{Entity, EntityHandleRuntime, WeakEntity},
};
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

/// The read operations supplied by an application context implementation.
///
/// The associated entity family lets each backend retain its concrete entity
/// handle type while sharing the operation contract.
pub trait AppContextRead {
    /// The entity handle family used by this application context.
    type Entity<T>;
    /// The application context passed to read callbacks.
    type App;

    /// Reads an entity through the application context.
    fn read_entity<T, R>(
        &self,
        entity: &Self::Entity<T>,
        read: impl FnOnce(&T, &Self::App) -> R,
    ) -> R
    where
        T: 'static;

    /// Reads a global through the application context.
    fn read_global<G, R>(&self, read: impl FnOnce(&G, &Self::App) -> R) -> R
    where
        G: Global;

    /// Tries to read a global through the application context.
    fn try_read_global<G, R>(&self, read: impl FnOnce(&G, &Self::App) -> R) -> Option<R>
    where
        G: Global;
}

/// The update operations supplied by an application context implementation.
pub trait AppContextUpdate: AppContextRead {
    /// The context family supplied while updating an entity.
    type Context<'a, T>
    where
        Self: 'a;

    /// Updates an entity through the application context.
    fn update_entity<T, R>(
        &mut self,
        entity: &Self::Entity<T>,
        update: impl FnOnce(&mut T, &mut Self::Context<'_, T>) -> R,
    ) -> R
    where
        T: 'static;

    /// Updates a global through the application context.
    fn update_global<G, R>(&mut self, update: impl FnOnce(&mut G, &mut Self::App) -> R) -> R
    where
        G: Global;

    /// Sets a global through the application context.
    fn set_global<G: Global>(&mut self, global: G);

    /// Updates a global, assigning its default value when necessary.
    fn update_default_global<G, R>(
        &mut self,
        update: impl FnOnce(&mut G, &mut Self::App) -> R,
    ) -> R
    where
        G: Global + Default;
}

/// The global observation operations supplied by an application context.
pub trait AppContextObserve {
    /// The application context passed to observation callbacks.
    type App;
    /// The subscription handle returned by this context.
    type Subscription: SubscriptionHandle;

    /// Observes updates to a global through the application context.
    fn observe_global<G>(
        &mut self,
        on_update: impl FnMut(&mut Self::App) + 'static,
    ) -> Self::Subscription
    where
        G: Global;
}

/// The observation operations supplied by an entity context implementation.
pub trait ContextObserve<T> {
    /// The entity handle family used by this context.
    type Entity<U>;
    /// The subscription handle returned by this context.
    type Subscription: SubscriptionHandle;

    /// Observes notifications from another entity.
    fn observe<W>(
        &mut self,
        entity: &Self::Entity<W>,
        on_notify: impl FnMut(&mut T, Self::Entity<W>) + 'static,
    ) -> Self::Subscription
    where
        T: 'static,
        W: 'static;

    /// Observes updates to a global through this entity context.
    fn observe_global<G>(&mut self, on_update: impl FnMut(&mut T) + 'static) -> Self::Subscription
    where
        T: 'static,
        G: 'static;
}

/// The foreground spawn operation supplied by an application context.
pub trait AppContextSpawn {
    /// The asynchronous application context supplied to spawned callbacks.
    type AsyncContext;
    /// The task handle returned by this context.
    type Task<T>: TaskHandle<T>;

    /// Spawns a future on the foreground executor.
    fn spawn<AsyncFn, R>(&self, callback: AsyncFn) -> Self::Task<R>
    where
        AsyncFn: AsyncFnOnce(&mut Self::AsyncContext) -> R + 'static,
        R: 'static;
}

/// The foreground spawn operation supplied by an entity context.
pub trait ContextSpawn<T> {
    /// The weak handle supplied to spawned callbacks.
    type WeakEntity: super::WeakEntityHandle<T>;
    /// The asynchronous application context supplied to spawned callbacks.
    type AsyncContext;
    /// The task handle returned by this context.
    type Task<R>: TaskHandle<R>;

    /// Spawns a future associated with this entity.
    fn spawn<AsyncFn, R>(&self, callback: AsyncFn) -> Self::Task<R>
    where
        AsyncFn: AsyncFnOnce(Self::WeakEntity, &mut Self::AsyncContext) -> R + 'static,
        R: 'static;
}

/// The entity operations supplied by an application context.
pub trait ContextSpi {
    /// The strong handle type returned by [`Self::entity`].
    type Entity: EntityHandle;

    /// The weak handle type returned by [`Self::weak_entity`].
    type WeakEntity: EntityHandle;

    /// Returns the identifier of the entity associated with this context.
    fn entity_id(&self) -> EntityId;

    /// Attempts to obtain a strong handle for the entity associated with this context.
    fn entity(&self) -> Option<Self::Entity>;

    /// Returns a weak handle for the entity associated with this context.
    fn weak_entity(&self) -> Self::WeakEntity;
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

impl<T: 'static> ContextSpi for Context<'_, T> {
    type Entity = Entity<T>;
    type WeakEntity = WeakEntity<T>;

    fn entity_id(&self) -> EntityId {
        self.entity_id()
    }

    fn entity(&self) -> Option<Self::Entity> {
        self.entity()
    }

    fn weak_entity(&self) -> Self::WeakEntity {
        self.weak_entity()
    }
}
