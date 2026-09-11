pub use gpui_types::{
    AppContextObserve, AppContextRead, AppContextSpawn, AppContextSpi, AppContextUpdate,
    AppContextWindow, ContextObserve, ContextSpawn, EntityHandle, EntityStorageSpi,
    StrongEntityHandle, SubscriptionHandle, TaskHandle, VisualContextSpi, WeakEntityHandle,
};

use crate::{
    AnyEntity, AnyView, AnyWeakEntity, AnyWindowHandle, App, AsyncApp, AsyncWindowContext, Context,
    Entity, EntityId, Global, Subscription, Task, WeakEntity, Window, WindowHandle,
};

macro_rules! impl_app_context_window_spi {
    ($context:ty, $($impl_generics:tt)*) => {
        impl $($impl_generics)* AppContextWindow for $context {
            type Entity<T> = Entity<T>;
            type AnyView = AnyView;
            type Window = Window;
            type App = App;
            type AnyWindowHandle = AnyWindowHandle;
            type WindowHandle<T> = WindowHandle<T>;
            type WindowResult<T> = crate::Result<T>;

            fn spi_update_window<T, F>(
                &mut self,
                window: AnyWindowHandle,
                update: F,
            ) -> Self::WindowResult<T>
            where
                F: FnOnce(AnyView, &mut Window, &mut App) -> T,
            {
                <Self as crate::AppContext>::update_window(self, window, update)
            }

            fn spi_with_window<R>(
                &mut self,
                entity_id: EntityId,
                update: impl FnOnce(&mut Window, &mut App) -> R,
            ) -> Option<R> {
                <Self as crate::AppContext>::with_window(self, entity_id, update)
            }

            fn spi_read_window<T, R>(
                &self,
                window: &WindowHandle<T>,
                read: impl FnOnce(Entity<T>, &App) -> R,
            ) -> Self::WindowResult<R>
            where
                T: 'static,
            {
                <Self as crate::AppContext>::read_window(self, window, read)
            }
        }
    };
}

impl_app_context_window_spi!(App,);
impl_app_context_window_spi!(AsyncApp,);
impl_app_context_window_spi!(AsyncWindowContext,);
impl_app_context_window_spi!(Context<'_, ContextEntity>, <ContextEntity>);

impl VisualContextSpi for AsyncWindowContext {
    type VisualResult<T> = crate::Result<T>;
    type Context<'a, T> = Context<'a, T>;

    fn spi_window_handle(&self) -> AnyWindowHandle {
        <Self as crate::VisualContext>::window_handle(self)
    }

    fn spi_update_window_entity<T, R>(
        &mut self,
        entity: &Entity<T>,
        update: impl FnOnce(&mut T, &mut Window, &mut Context<'_, T>) -> R,
    ) -> Self::VisualResult<R>
    where
        T: 'static,
    {
        <Self as crate::VisualContext>::update_window_entity(self, entity, update)
    }

    fn spi_new_window_entity<T>(
        &mut self,
        build_entity: impl FnOnce(&mut Window, &mut Context<'_, T>) -> T,
    ) -> Self::VisualResult<Entity<T>>
    where
        T: 'static,
    {
        <Self as crate::VisualContext>::new_window_entity(self, build_entity)
    }
}

impl AppContextRead for App {
    type Entity<T> = Entity<T>;
    type App = App;

    fn read_entity<T, R>(&self, entity: &Entity<T>, read: impl FnOnce(&T, &App) -> R) -> R
    where
        T: 'static,
    {
        <App as crate::AppContext>::read_entity(self, entity, read)
    }

    fn read_global<G, R>(&self, read: impl FnOnce(&G, &App) -> R) -> R
    where
        G: Global,
    {
        <App as crate::AppContext>::read_global(self, read)
    }

    fn try_read_global<G, R>(&self, read: impl FnOnce(&G, &App) -> R) -> Option<R>
    where
        G: Global,
    {
        self.try_global().map(|global| read(global, self))
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

    fn update_global<G, R>(&mut self, update: impl FnOnce(&mut G, &mut App) -> R) -> R
    where
        G: Global,
    {
        crate::BorrowAppContext::update_global(self, update)
    }

    fn set_global<G: Global>(&mut self, global: G) {
        crate::BorrowAppContext::set_global(self, global)
    }

    fn update_default_global<G, R>(&mut self, update: impl FnOnce(&mut G, &mut App) -> R) -> R
    where
        G: Global + Default,
    {
        crate::BorrowAppContext::update_default_global(self, update)
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

    fn read_global<G, R>(&self, read: impl FnOnce(&G, &App) -> R) -> R
    where
        G: Global,
    {
        <AsyncApp as crate::AppContext>::read_global(self, read)
    }

    fn try_read_global<G, R>(&self, read: impl FnOnce(&G, &App) -> R) -> Option<R>
    where
        G: Global,
    {
        AsyncApp::try_read_global(self, read)
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

    fn update_global<G, R>(&mut self, update: impl FnOnce(&mut G, &mut App) -> R) -> R
    where
        G: Global,
    {
        AsyncApp::update_global(self, update)
    }

    fn set_global<G: Global>(&mut self, global: G) {
        self.update(|cx| cx.set_global(global));
    }

    fn update_default_global<G, R>(&mut self, update: impl FnOnce(&mut G, &mut App) -> R) -> R
    where
        G: Global + Default,
    {
        self.update(|cx| cx.update_default_global(update))
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

    fn read_global<G, R>(&self, read: impl FnOnce(&G, &App) -> R) -> R
    where
        G: Global,
    {
        <AsyncWindowContext as crate::AppContext>::read_global(self, read)
    }

    fn try_read_global<G, R>(&self, read: impl FnOnce(&G, &App) -> R) -> Option<R>
    where
        G: Global,
    {
        AsyncApp::try_read_global(self, read)
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

    fn update_global<G, R>(&mut self, update: impl FnOnce(&mut G, &mut App) -> R) -> R
    where
        G: Global,
    {
        AsyncApp::update_global(self, update)
    }

    fn set_global<G: Global>(&mut self, global: G) {
        AsyncApp::update(self, |cx| cx.set_global(global));
    }

    fn update_default_global<G, R>(&mut self, update: impl FnOnce(&mut G, &mut App) -> R) -> R
    where
        G: Global + Default,
    {
        AsyncApp::update(self, |cx| cx.update_default_global(update))
    }
}

impl AppContextObserve for App {
    type App = App;
    type Subscription = Subscription;

    fn observe_global<G>(&mut self, on_update: impl FnMut(&mut App) + 'static) -> Self::Subscription
    where
        G: Global,
    {
        App::observe_global::<G>(self, on_update)
    }
}

impl AppContextObserve for AsyncApp {
    type App = App;
    type Subscription = Subscription;

    fn observe_global<G>(&mut self, on_update: impl FnMut(&mut App) + 'static) -> Self::Subscription
    where
        G: Global,
    {
        self.update(|cx| cx.observe_global::<G>(on_update))
    }
}

impl AppContextObserve for AsyncWindowContext {
    type App = App;
    type Subscription = Subscription;

    fn observe_global<G>(&mut self, on_update: impl FnMut(&mut App) + 'static) -> Self::Subscription
    where
        G: Global,
    {
        AsyncApp::update(self, |cx| cx.observe_global::<G>(on_update))
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

    fn observe_global<G>(&mut self, on_update: impl FnMut(&mut T) + 'static) -> Self::Subscription
    where
        G: 'static,
    {
        let mut on_update = on_update;
        Context::observe_global::<G>(self, move |state, _context| on_update(state))
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
    const fn assert_app_observe<T: AppContextObserve>() {}
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
    assert_app_observe::<App>();
    assert_app_observe::<AsyncApp>();
    assert_app_observe::<AsyncWindowContext>();
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
