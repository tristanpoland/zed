pub use gpui_types::{
    AppContextCore, AppContextObserve, AppContextRead, AppContextSpawn, AppContextSpi,
    AppContextUpdate, AppContextWindow, ContextListener, ContextObserve, ContextSpawn,
    EntityHandle, EntityStorageSpi, StrongEntityHandle, SubscriptionHandle, TaskHandle,
    VisualContextSpi, WeakEntityHandle,
};

use crate::{
    AnyEntity, AnyWeakEntity, App, AppContext, AsyncApp, AsyncWindowContext, Context, Entity,
    EntityId, Reservation, Subscription, Task, WeakEntity,
};

impl AppContextCore for App {
    type Entity<T> = Entity<T>;
    type Reservation<T> = Reservation<T>;
    type Context<'a, T> = Context<'a, T>;
    type App = App;
    type Task<T> = Task<T>;

    fn new<T: 'static>(
        &mut self,
        build_entity: impl FnOnce(&mut Self::Context<'_, T>) -> T,
    ) -> Self::Entity<T> {
        <Self as AppContext>::new(self, build_entity)
    }

    fn reserve_entity<T: 'static>(&mut self) -> Self::Reservation<T> {
        <Self as AppContext>::reserve_entity(self)
    }

    fn insert_entity<T: 'static>(
        &mut self,
        reservation: Self::Reservation<T>,
        build_entity: impl FnOnce(&mut Self::Context<'_, T>) -> T,
    ) -> Self::Entity<T> {
        <Self as AppContext>::insert_entity(self, reservation, build_entity)
    }

    fn update_entity<T: 'static, R>(
        &mut self,
        entity: &Self::Entity<T>,
        update: impl FnOnce(&mut T, &mut Self::Context<'_, T>) -> R,
    ) -> R {
        <Self as AppContext>::update_entity(self, entity, update)
    }

    fn read_entity<T: 'static, R>(
        &self,
        entity: &Self::Entity<T>,
        read: impl FnOnce(&T, &Self::App) -> R,
    ) -> R {
        <Self as AppContext>::read_entity(self, entity, read)
    }

    fn background_spawn<R>(
        &self,
        future: impl std::future::Future<Output = R> + Send + 'static,
    ) -> Self::Task<R>
    where
        R: Send + 'static,
    {
        <Self as AppContext>::background_spawn(self, future)
    }
}

impl AppContextCore for AsyncApp {
    type Entity<T> = Entity<T>;
    type Reservation<T> = Reservation<T>;
    type Context<'a, T> = Context<'a, T>;
    type App = App;
    type Task<T> = Task<T>;

    fn new<T: 'static>(
        &mut self,
        build_entity: impl FnOnce(&mut Self::Context<'_, T>) -> T,
    ) -> Self::Entity<T> {
        <Self as AppContext>::new(self, build_entity)
    }

    fn reserve_entity<T: 'static>(&mut self) -> Self::Reservation<T> {
        <Self as AppContext>::reserve_entity(self)
    }

    fn insert_entity<T: 'static>(
        &mut self,
        reservation: Self::Reservation<T>,
        build_entity: impl FnOnce(&mut Self::Context<'_, T>) -> T,
    ) -> Self::Entity<T> {
        <Self as AppContext>::insert_entity(self, reservation, build_entity)
    }

    fn update_entity<T: 'static, R>(
        &mut self,
        entity: &Self::Entity<T>,
        update: impl FnOnce(&mut T, &mut Self::Context<'_, T>) -> R,
    ) -> R {
        <Self as AppContext>::update_entity(self, entity, update)
    }

    fn read_entity<T: 'static, R>(
        &self,
        entity: &Self::Entity<T>,
        read: impl FnOnce(&T, &Self::App) -> R,
    ) -> R {
        <Self as AppContext>::read_entity(self, entity, read)
    }

    fn background_spawn<R>(
        &self,
        future: impl std::future::Future<Output = R> + Send + 'static,
    ) -> Self::Task<R>
    where
        R: Send + 'static,
    {
        <Self as AppContext>::background_spawn(self, future)
    }
}

impl AppContextCore for AsyncWindowContext {
    type Entity<T> = Entity<T>;
    type Reservation<T> = Reservation<T>;
    type Context<'a, T> = Context<'a, T>;
    type App = App;
    type Task<T> = Task<T>;

    fn new<T: 'static>(
        &mut self,
        build_entity: impl FnOnce(&mut Self::Context<'_, T>) -> T,
    ) -> Self::Entity<T> {
        <Self as AppContext>::new(self, build_entity)
    }

    fn reserve_entity<T: 'static>(&mut self) -> Self::Reservation<T> {
        <Self as AppContext>::reserve_entity(self)
    }

    fn insert_entity<T: 'static>(
        &mut self,
        reservation: Self::Reservation<T>,
        build_entity: impl FnOnce(&mut Self::Context<'_, T>) -> T,
    ) -> Self::Entity<T> {
        <Self as AppContext>::insert_entity(self, reservation, build_entity)
    }

    fn update_entity<T: 'static, R>(
        &mut self,
        entity: &Self::Entity<T>,
        update: impl FnOnce(&mut T, &mut Self::Context<'_, T>) -> R,
    ) -> R {
        <Self as AppContext>::update_entity(self, entity, update)
    }

    fn read_entity<T: 'static, R>(
        &self,
        entity: &Self::Entity<T>,
        read: impl FnOnce(&T, &Self::App) -> R,
    ) -> R {
        <Self as AppContext>::read_entity(self, entity, read)
    }

    fn background_spawn<R>(
        &self,
        future: impl std::future::Future<Output = R> + Send + 'static,
    ) -> Self::Task<R>
    where
        R: Send + 'static,
    {
        <Self as AppContext>::background_spawn(self, future)
    }
}

impl AppContextSpi for App {
    fn entity_storage(&self) -> &dyn EntityStorageSpi {
        &self.entities
    }
}

impl<T: 'static> EntityHandle for Entity<T> {
    fn entity_id(&self) -> EntityId {
        Entity::entity_id(self)
    }
}

impl<T: 'static> StrongEntityHandle<T> for Entity<T> {
    type Weak = WeakEntity<T>;

    fn downgrade(&self) -> Self::Weak {
        Entity::downgrade(self)
    }
}

impl<T> EntityHandle for WeakEntity<T> {
    fn entity_id(&self) -> EntityId {
        std::ops::Deref::deref(self).entity_id()
    }
}

impl<T: 'static> WeakEntityHandle<T> for WeakEntity<T> {
    type Strong = Entity<T>;

    fn upgrade(&self) -> Option<Self::Strong> {
        WeakEntity::upgrade(self)
    }
}

impl EntityHandle for AnyEntity {
    fn entity_id(&self) -> EntityId {
        AnyEntity::entity_id(self)
    }
}

impl EntityHandle for AnyWeakEntity {
    fn entity_id(&self) -> EntityId {
        AnyWeakEntity::entity_id(self)
    }
}

impl SubscriptionHandle for Subscription {
    fn detach(self) {
        Subscription::detach(self)
    }
}

const _: () = {
    const fn assert_app_core<T: AppContextCore>() {}
    const fn assert_app_context<T: AppContextSpi>() {}
    const fn assert_entity_handle<T: EntityHandle>() {}
    const fn assert_strong_entity_handle<T: StrongEntityHandle<()>>() {}
    const fn assert_weak_entity_handle<T: WeakEntityHandle<()>>() {}
    const fn assert_task_handle<T: TaskHandle<()>>() {}

    assert_app_core::<App>();
    assert_app_core::<AsyncApp>();
    assert_app_core::<AsyncWindowContext>();
    assert_app_context::<App>();
    assert_entity_handle::<Entity<()>>();
    assert_entity_handle::<WeakEntity<()>>();
    assert_entity_handle::<AnyEntity>();
    assert_entity_handle::<AnyWeakEntity>();
    assert_strong_entity_handle::<Entity<()>>();
    assert_weak_entity_handle::<WeakEntity<()>>();
    assert_task_handle::<Task<()>>();
};
