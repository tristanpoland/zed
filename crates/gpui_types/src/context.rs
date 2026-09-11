use super::{
    EntityHandle, EntityId, EventEmitter, Global, SubscriptionHandle, TaskHandle,
    entity::{Entity, EntityHandleRuntime, WeakEntity},
};
use std::{
    future::Future,
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

/// The window operations supplied by an application context implementation.
///
/// The concrete window, view, and handle types remain implementation-owned;
/// this contract only describes how an app context exposes them to the
/// backend-neutral API boundary.
pub trait AppContextWindow {
    /// The entity handle family used by this application context.
    type Entity<T>;
    /// The type-erased view passed to window update callbacks.
    type AnyView;
    /// The window type passed to window callbacks.
    type Window;
    /// The application context passed to window callbacks.
    type App;
    /// The typed entity context passed to window callbacks.
    type WindowContext<'a, T>
    where
        Self: 'a;
    /// The type-erased window handle.
    type AnyWindowHandle: Copy;
    /// The typed window handle family.
    type WindowHandle<T>;
    /// The result returned by window operations.
    type WindowResult<T>;

    /// Updates a window and passes its root view to the callback.
    fn spi_update_window<T, F>(
        &mut self,
        window: Self::AnyWindowHandle,
        update: F,
    ) -> Self::WindowResult<T>
    where
        F: FnOnce(Self::AnyView, &mut Self::Window, &mut Self::App) -> T;

    /// Updates a window's typed root entity.
    fn spi_update_window_entity<T, R>(
        &mut self,
        window: &crate::WindowHandle<T>,
        update: impl FnOnce(&mut T, &mut Self::Window, &mut Self::WindowContext<'_, T>) -> R,
    ) -> Self::WindowResult<R>
    where
        T: 'static;

    /// Runs a callback against the current window for an entity, if available.
    fn spi_with_window<R>(
        &mut self,
        entity_id: EntityId,
        update: impl FnOnce(&mut Self::Window, &mut Self::App) -> R,
    ) -> Option<R>;

    /// Reads the typed root entity of a window.
    fn spi_read_window<T, R>(
        &self,
        window: &Self::WindowHandle<T>,
        read: impl FnOnce(Self::Entity<T>, &Self::App) -> R,
    ) -> Self::WindowResult<R>
    where
        T: 'static;

    /// Reads a typed root entity through a type-erased window handle.
    fn spi_read_window_any<T, R>(
        &self,
        window: crate::AnyWindowHandle,
        read: impl FnOnce(Self::Entity<T>, &Self::App) -> R,
    ) -> Self::WindowResult<R>
    where
        T: 'static;

    /// Reads a window's typed root entity through a callback.
    fn spi_read_window_root_with<T, R>(
        &self,
        window: &crate::WindowHandle<T>,
        read: impl FnOnce(&T, &Self::App) -> R,
    ) -> Self::WindowResult<R>
    where
        T: 'static;

    /// Returns whether a window is active, or `None` if it is unavailable.
    fn spi_window_is_active(&mut self, window: crate::AnyWindowHandle) -> Option<bool>;
}

/// The root-view read capability supplied by an application context.
pub trait WindowRootReadSpi: AppContextWindow {
    /// Reads a typed root view while retaining the context borrow.
    fn spi_read_window_root<'a, T>(
        &'a self,
        window: &crate::WindowHandle<T>,
    ) -> Self::WindowResult<&'a T>
    where
        T: 'static;
}

/// The window/entity operations supplied by a visual context implementation.
pub trait VisualContextSpi: AppContextWindow {
    /// The result returned by visual-context operations.
    type VisualResult<T>;
    /// The entity context family supplied to visual callbacks.
    type Context<'a, T>
    where
        Self: 'a;

    /// Returns the window associated with this visual context.
    fn spi_window_handle(&self) -> Self::AnyWindowHandle;

    /// Updates an entity with access to its current window.
    fn spi_update_window_entity<T, R>(
        &mut self,
        entity: &Self::Entity<T>,
        update: impl FnOnce(&mut T, &mut Self::Window, &mut Self::Context<'_, T>) -> R,
    ) -> Self::VisualResult<R>
    where
        T: 'static;

    /// Creates an entity with access to this context's window.
    fn spi_new_window_entity<T>(
        &mut self,
        build_entity: impl FnOnce(&mut Self::Window, &mut Self::Context<'_, T>) -> T,
    ) -> Self::VisualResult<Self::Entity<T>>
    where
        T: 'static;
}

/// The global observation operations supplied by an application context.
pub trait AppContextObserve {
    /// The entity handle family used by this application context.
    type Entity<T>;
    /// The application context passed to observation callbacks.
    type App;
    /// The subscription handle returned by this context.
    type Subscription: SubscriptionHandle;

    /// Observes notifications from an entity.
    fn observe<W>(
        &mut self,
        entity: &Self::Entity<W>,
        on_notify: impl FnMut(Self::Entity<W>, &mut Self::App) + 'static,
    ) -> Self::Subscription
    where
        W: 'static;

    /// Subscribes to events emitted by an entity.
    fn subscribe<T, Event>(
        &mut self,
        entity: &Self::Entity<T>,
        on_event: impl FnMut(Self::Entity<T>, &Event, &mut Self::App) + 'static,
    ) -> Self::Subscription
    where
        T: 'static + EventEmitter<Event>,
        Event: 'static;

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

    /// Subscribes to events emitted by another entity.
    fn subscribe<W, Event>(
        &mut self,
        entity: &Self::Entity<W>,
        on_event: impl FnMut(&mut T, Self::Entity<W>, &Event) + 'static,
    ) -> Self::Subscription
    where
        T: 'static,
        W: 'static + EventEmitter<Event>,
        Event: 'static;

    /// Subscribes to events emitted by this context's entity.
    fn subscribe_self<Event>(
        &mut self,
        on_event: impl FnMut(&mut T, &Event) + 'static,
    ) -> Self::Subscription
    where
        T: 'static + EventEmitter<Event>,
        Event: 'static;

    /// Observes updates to a global through this entity context.
    fn observe_global<G>(&mut self, on_update: impl FnMut(&mut T) + 'static) -> Self::Subscription
    where
        T: 'static,
        G: 'static;
}

/// The event callback helpers supplied by an entity context implementation.
pub trait ContextListener<T> {
    /// The window type passed to event callbacks.
    type Window;
    /// The application context passed to event callbacks.
    type App;
    /// The entity context passed to event callbacks.
    type Context<'a>;

    /// Builds a callback that gives an event handler access to entity state.
    fn listener<E: ?Sized>(
        &self,
        callback: impl for<'a> Fn(&mut T, &E, &mut Self::Window, &mut Self::Context<'a>) + 'static,
    ) -> impl Fn(&E, &mut Self::Window, &mut Self::App) + 'static;

    /// Builds a callback that returns a value from an event handler.
    fn processor<E, R>(
        &self,
        callback: impl for<'a> Fn(&mut T, E, &mut Self::Window, &mut Self::Context<'a>) -> R + 'static,
    ) -> impl Fn(E, &mut Self::Window, &mut Self::App) -> R + 'static;
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

    /// Spawns a future on a background executor.
    fn background_spawn<R>(
        &self,
        future: impl Future<Output = R> + Send + 'static,
    ) -> Self::Task<R>
    where
        R: Send + 'static;
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

/// The backend-neutral core of an application context.
///
/// This is the largest context family that can cross the API boundary without
/// naming GPUI's window, view, or mutable-borrow types. Window operations stay
/// in [`AppContextWindow`] until those identities are shared as well.
pub trait AppContextCore {
    /// The strong entity handle family returned by this context.
    type Entity<T>;
    /// The reservation family returned by this context.
    type Reservation<T>;
    /// The entity context family supplied to callbacks.
    type Context<'a, T>
    where
        Self: 'a;
    /// The application context passed to read callbacks.
    type App;
    /// The task handle family returned by background work.
    type Task<T>: TaskHandle<T>;

    /// Creates an entity owned by this application context.
    fn new<T: 'static>(
        &mut self,
        build_entity: impl FnOnce(&mut Self::Context<'_, T>) -> T,
    ) -> Self::Entity<T>;

    /// Reserves an entity slot for later insertion.
    fn reserve_entity<T: 'static>(&mut self) -> Self::Reservation<T>;

    /// Inserts an entity into a previously reserved slot.
    fn insert_entity<T: 'static>(
        &mut self,
        reservation: Self::Reservation<T>,
        build_entity: impl FnOnce(&mut Self::Context<'_, T>) -> T,
    ) -> Self::Entity<T>;

    /// Updates an entity through this application context.
    fn update_entity<T: 'static, R>(
        &mut self,
        entity: &Self::Entity<T>,
        update: impl FnOnce(&mut T, &mut Self::Context<'_, T>) -> R,
    ) -> R;

    /// Reads an entity through this application context.
    fn read_entity<T: 'static, R>(
        &self,
        entity: &Self::Entity<T>,
        read: impl FnOnce(&T, &Self::App) -> R,
    ) -> R;

    /// Spawns a future on this context's background executor.
    fn background_spawn<R>(
        &self,
        future: impl Future<Output = R> + Send + 'static,
    ) -> Self::Task<R>
    where
        R: Send + 'static;
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
