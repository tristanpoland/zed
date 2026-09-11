pub use gpui_types::{
    AppContextObserve, AppContextRead, AppContextSpawn, AppContextSpi, AppContextUpdate,
    AppContextWindow, ContextListener, ContextObserve, ContextSpawn, EntityHandle,
    EntityStorageSpi, StrongEntityHandle, SubscriptionHandle, TaskHandle, VisualContextSpi,
    WeakEntityHandle,
};

use crate::{
    AnyEntity, AnyView, AnyWeakEntity, AnyWindowHandle, App, AsyncApp, AsyncWindowContext, Context,
    Entity, EntityId, Subscription, Task, WeakEntity, Window, WindowHandle,
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
    const fn assert_entity_handle<T: EntityHandle>() {}
    const fn assert_strong_entity_handle<T: StrongEntityHandle<()>>() {}
    const fn assert_weak_entity_handle<T: WeakEntityHandle<()>>() {}
    const fn assert_task_handle<T: TaskHandle<()>>() {}

    assert_app_context::<App>();
    assert_entity_handle::<Entity<()>>();
    assert_entity_handle::<WeakEntity<()>>();
    assert_entity_handle::<AnyEntity>();
    assert_entity_handle::<AnyWeakEntity>();
    assert_strong_entity_handle::<Entity<()>>();
    assert_weak_entity_handle::<WeakEntity<()>>();
    assert_task_handle::<Task<()>>();
};
