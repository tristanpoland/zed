pub use gpui_types::{
    AppContextRead, AppContextSpawn, AppContextSpi, AppContextUpdate, ContextObserve, ContextSpawn,
    EntityHandle, EntityStorageSpi, StrongEntityHandle, SubscriptionHandle, TaskHandle,
    WeakEntityHandle,
};

use crate::{
    AnyEntity, AnyWeakEntity, App, AsyncApp, AsyncWindowContext, Context, Entity, EntityId,
    Subscription, Task, WeakEntity,
};

impl AppContextRead for App {
    type Entity<T> = Entity<T>;
    type App = App;

    fn read_entity<T, R>(&self, entity: &Entity<T>, read: impl FnOnce(&T, &App) -> R) -> R
    where
        T: 'static,
    {
        <App as crate::AppContext>::read_entity(self, entity, read)
    }
}

impl AppContextUpdate for App {
    type Context<'a, T> = Context<'a, T>;

    fn update_entity<T, R>(
        &mut self,
        entity: &Entity<T>,
        update: impl FnOnce(&mut T, &mut Context<'_, T>) -> R,
    ) -> R
    where
        T: 'static,
    {
        <App as crate::AppContext>::update_entity(self, entity, update)
    }
}

impl AppContextRead for AsyncApp {
    type Entity<T> = Entity<T>;
    type App = App;

    fn read_entity<T, R>(&self, entity: &Entity<T>, read: impl FnOnce(&T, &App) -> R) -> R
    where
        T: 'static,
    {
        <AsyncApp as crate::AppContext>::read_entity(self, entity, read)
    }
}

impl AppContextUpdate for AsyncApp {
    type Context<'a, T> = Context<'a, T>;

    fn update_entity<T, R>(
        &mut self,
        entity: &Entity<T>,
        update: impl FnOnce(&mut T, &mut Context<'_, T>) -> R,
    ) -> R
    where
        T: 'static,
    {
        <AsyncApp as crate::AppContext>::update_entity(self, entity, update)
    }
}

impl AppContextRead for AsyncWindowContext {
    type Entity<T> = Entity<T>;
    type App = App;

    fn read_entity<T, R>(&self, entity: &Entity<T>, read: impl FnOnce(&T, &App) -> R) -> R
    where
        T: 'static,
    {
        <AsyncWindowContext as crate::AppContext>::read_entity(self, entity, read)
    }
}

impl AppContextUpdate for AsyncWindowContext {
    type Context<'a, T> = Context<'a, T>;

    fn update_entity<T, R>(
        &mut self,
        entity: &Entity<T>,
        update: impl FnOnce(&mut T, &mut Context<'_, T>) -> R,
    ) -> R
    where
        T: 'static,
    {
        <AsyncWindowContext as crate::AppContext>::update_entity(self, entity, update)
    }
}

impl AppContextSpawn for App {
    type AsyncContext = AsyncApp;
    type Task<T> = Task<T>;

    fn spawn<AsyncFn, R>(&self, callback: AsyncFn) -> Self::Task<R>
    where
        AsyncFn: AsyncFnOnce(&mut Self::AsyncContext) -> R + 'static,
        R: 'static,
    {
        App::spawn(self, callback)
    }
}

impl AppContextSpawn for AsyncApp {
    type AsyncContext = AsyncApp;
    type Task<T> = Task<T>;

    fn spawn<AsyncFn, R>(&self, callback: AsyncFn) -> Self::Task<R>
    where
        AsyncFn: AsyncFnOnce(&mut Self::AsyncContext) -> R + 'static,
        R: 'static,
    {
        AsyncApp::spawn(self, callback)
    }
}

impl AppContextSpawn for AsyncWindowContext {
    type AsyncContext = AsyncWindowContext;
    type Task<T> = Task<T>;

    fn spawn<AsyncFn, R>(&self, callback: AsyncFn) -> Self::Task<R>
    where
        AsyncFn: AsyncFnOnce(&mut Self::AsyncContext) -> R + 'static,
        R: 'static,
    {
        AsyncWindowContext::spawn(self, callback)
    }
}

impl<T: 'static> ContextObserve<T> for Context<'_, T> {
    type Entity<U> = Entity<U>;
    type Subscription = Subscription;

    fn observe<W>(
        &mut self,
        entity: &Entity<W>,
        on_notify: impl FnMut(&mut T, Entity<W>) + 'static,
    ) -> Self::Subscription
    where
        W: 'static,
    {
        let mut on_notify = on_notify;
        Context::observe(self, entity, move |state, entity, _context| {
            on_notify(state, entity)
        })
    }
}

impl<T: 'static> ContextSpawn<T> for Context<'_, T> {
    type WeakEntity = WeakEntity<T>;
    type AsyncContext = AsyncApp;
    type Task<R> = Task<R>;

    fn spawn<AsyncFn, R>(&self, callback: AsyncFn) -> Self::Task<R>
    where
        AsyncFn: AsyncFnOnce(Self::WeakEntity, &mut Self::AsyncContext) -> R + 'static,
        R: 'static,
    {
        Context::spawn(self, callback)
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

impl<T> TaskHandle<T> for Task<T> {
    fn detach(self) {
        Task::detach(self)
    }
}

const _: () = {
    const fn assert_app_context<T: AppContextSpi>() {}
    const fn assert_app_read<T: AppContextRead>() {}
    const fn assert_app_update<T: AppContextUpdate>() {}
    const fn assert_app_spawn<T: AppContextSpawn>() {}
    const fn assert_context_observe<T: ContextObserve<()>>() {}
    const fn assert_context_spawn<T: ContextSpawn<()>>() {}
    const fn assert_entity_handle<T: EntityHandle>() {}
    const fn assert_strong_entity_handle<T: StrongEntityHandle<()>>() {}
    const fn assert_weak_entity_handle<T: WeakEntityHandle<()>>() {}
    const fn assert_task_handle<T: TaskHandle<()>>() {}

    assert_app_context::<App>();
    assert_app_read::<App>();
    assert_app_read::<AsyncApp>();
    assert_app_read::<AsyncWindowContext>();
    assert_app_update::<App>();
    assert_app_update::<AsyncApp>();
    assert_app_update::<AsyncWindowContext>();
    assert_app_spawn::<App>();
    assert_app_spawn::<AsyncApp>();
    assert_app_spawn::<AsyncWindowContext>();
    assert_context_observe::<Context<'static, ()>>();
    assert_context_spawn::<Context<'static, ()>>();
    assert_entity_handle::<Entity<()>>();
    assert_entity_handle::<WeakEntity<()>>();
    assert_entity_handle::<AnyEntity>();
    assert_entity_handle::<AnyWeakEntity>();
    assert_strong_entity_handle::<Entity<()>>();
    assert_weak_entity_handle::<WeakEntity<()>>();
    assert_task_handle::<Task<()>>();
};
