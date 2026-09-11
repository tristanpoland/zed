pub use gpui_types::{
    AppContextCore, AppContextObserve, AppContextRead, AppContextSpawn, AppContextSpi,
    AppContextUpdate, AppContextWindow, ContextListener, ContextObserve, ContextSpawn,
    EntityHandle, EntityStorageSpi, StrongEntityHandle, SubscriptionHandle, TaskHandle,
    VisualContextSpi, WeakEntityHandle, WindowRootReadSpi,
};

use crate::{
    AnyEntity, AnyView, AnyWeakEntity, AnyWindowHandle, App, AppContext, AsyncApp,
    AsyncWindowContext, Context, Entity, EntityId, Reservation, Subscription, Task, WeakEntity,
    Window, WindowHandle,
};

#[cfg(feature = "bench-support")]
use crate::{BenchAppContext, BenchWindowContext};
#[cfg(any(test, feature = "test-support"))]
use crate::{HeadlessAppContext, TestAppContext, VisualTestContext};
#[cfg(all(target_os = "macos", any(test, feature = "test-support")))]
use crate::VisualTestAppContext;

macro_rules! impl_app_context_window_spi {
    ($context:ty, $($impl_generics:tt)*) => {
        impl $($impl_generics)* AppContextWindow for $context {
            type Entity<T> = Entity<T>;
            type AnyView = AnyView;
            type Window = Window;
            type App = App;
            type WindowContext<'a, T> = Context<'a, T> where Self: 'a;
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

            fn spi_update_window_entity<T, R>(
                &mut self,
                window: &WindowHandle<T>,
                update: impl FnOnce(&mut T, &mut Window, &mut Context<T>) -> R,
            ) -> Self::WindowResult<R>
            where
                T: 'static,
            {
                <Self as crate::AppContext>::update_window(self, (*window).into(), |root_view, window, cx| {
                    let view = root_view
                        .downcast::<T>()
                        .map_err(|_| anyhow::anyhow!("the type of the window's root view has changed"))?;
                    Ok(view.update(cx, |view, cx| update(view, window, cx)))
                })?
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

            fn spi_read_window_any<T, R>(
                &self,
                window: AnyWindowHandle,
                read: impl FnOnce(Entity<T>, &App) -> R,
            ) -> Self::WindowResult<R>
            where
                T: 'static,
            {
                let window = window
                    .downcast::<T>()
                    .ok_or_else(|| anyhow::anyhow!("the type of the window's root view has changed"))?;
                <Self as crate::AppContext>::read_window(self, &window, read)
            }

            fn spi_read_window_root_with<T, R>(
                &self,
                window: &WindowHandle<T>,
                read: impl FnOnce(&T, &App) -> R,
            ) -> Self::WindowResult<R>
            where
                T: 'static,
            {
                <Self as crate::AppContext>::read_window(self, window, |root_view, cx| {
                    read(root_view.read(cx), cx)
                })
            }

            fn spi_window_is_active(&mut self, window: AnyWindowHandle) -> Option<bool> {
                <Self as crate::AppContext>::update_window(self, window, |_, window, _| {
                    window.is_window_active()
                })
                .ok()
            }
        }
    };
}

impl WindowRootReadSpi for App {
    fn spi_read_window_root<'a, T>(
        &'a self,
        window: &WindowHandle<T>,
    ) -> <Self as AppContextWindow>::WindowResult<&'a T>
    where
        T: 'static,
    {
        let root_view = self
            .windows
            .get(window.window_id())
            .and_then(|window| {
                window
                    .as_deref()
                    .and_then(|window| window.root.clone())
                    .map(|root_view| root_view.downcast::<T>())
            })
            .ok_or_else(|| anyhow::anyhow!("window not found"))?
            .map_err(|_| anyhow::anyhow!("the type of the window's root view has changed"))?;

        Ok(root_view.read(self))
    }
}

impl_app_context_window_spi!(App,);
impl_app_context_window_spi!(AsyncApp,);
impl_app_context_window_spi!(AsyncWindowContext,);
impl_app_context_window_spi!(Context<'_, ContextEntity>, <ContextEntity>);
#[cfg(feature = "bench-support")]
impl_app_context_window_spi!(BenchAppContext<'_, '_>,);
#[cfg(feature = "bench-support")]
impl_app_context_window_spi!(BenchWindowContext<'_, '_>,);
#[cfg(any(test, feature = "test-support"))]
impl_app_context_window_spi!(HeadlessAppContext,);
#[cfg(any(test, feature = "test-support"))]
impl_app_context_window_spi!(TestAppContext,);
#[cfg(all(target_os = "macos", any(test, feature = "test-support")))]
impl_app_context_window_spi!(VisualTestAppContext,);
#[cfg(any(test, feature = "test-support"))]
impl_app_context_window_spi!(VisualTestContext,);

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
