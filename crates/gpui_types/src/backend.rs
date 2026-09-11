use crate::{
    AppContextObserve, AppContextRead, AppContextRuntime, AppContextSpawn, AppContextSpi,
    AppContextUpdate, AppContextWindow, EntityStorageSpi, PlatformServicesSpi, TaskHandle,
};
use std::{future::Future, sync::Arc, time::Duration};

/// The dispatcher capability used by a backend's task executors.
///
/// The runnable and priority types remain associated with the implementation so this contract
/// does not depend on a particular async runtime. A backend can therefore route the same
/// scheduler operations to a native event loop, a worker pool, or a deterministic test loop.
pub trait DispatcherSpi: Send + Sync + 'static {
    /// The runnable representation consumed by this dispatcher.
    type Runnable: Send + 'static;
    /// The priority representation consumed by this dispatcher.
    type Priority: Copy + Send + 'static;

    /// Enqueues background work with the given priority.
    fn dispatch(&self, runnable: Self::Runnable, priority: Self::Priority);

    /// Enqueues work on the foreground thread with the given priority.
    fn dispatch_on_main_thread(&self, runnable: Self::Runnable, priority: Self::Priority);

    /// Enqueues work to run after the given duration.
    fn dispatch_after(&self, duration: Duration, runnable: Self::Runnable);

    /// Enqueues work on the foreground thread during platform idle time.
    fn dispatch_on_main_thread_when_idle(
        &self,
        runnable: Self::Runnable,
        priority: Self::Priority,
        timeout: Option<Duration>,
    ) {
        let _ = timeout;
        self.dispatch_on_main_thread(runnable, priority);
    }

    /// Starts a dedicated thread for realtime work.
    fn spawn_realtime(&self, task: Box<dyn FnOnce() + Send>);
}

/// The background executor capability supplied by a backend runtime.
pub trait BackgroundExecutorSpi: Clone + 'static {
    /// The task handle returned by this executor.
    type Task<T>: TaskHandle<T>;
    /// The priority type accepted by this executor.
    type Priority: Copy + 'static;

    /// Enqueues a future on a background thread.
    fn spawn<R>(&self, future: impl Future<Output = R> + Send + 'static) -> Self::Task<R>
    where
        R: Send + 'static;

    /// Enqueues a future on a background thread with the given priority.
    fn spawn_with_priority<R>(
        &self,
        priority: Self::Priority,
        future: impl Future<Output = R> + Send + 'static,
    ) -> Self::Task<R>
    where
        R: Send + 'static;

    /// Returns a task that completes after the given duration.
    fn timer(&self, duration: Duration) -> Self::Task<()>;

    /// Returns the executor's current time representation.
    type Instant: Copy + 'static;

    /// Returns the current time used by this executor.
    fn now(&self) -> Self::Instant;
}

/// The foreground executor capability supplied by a backend runtime.
pub trait ForegroundExecutorSpi: Clone + 'static {
    /// The task handle returned by this executor.
    type Task<T>: TaskHandle<T>;
    /// The priority type accepted by this executor.
    type Priority: Copy + 'static;

    /// Enqueues a future on the foreground thread.
    fn spawn<R>(&self, future: impl Future<Output = R> + 'static) -> Self::Task<R>
    where
        R: 'static;

    /// Enqueues a future on the foreground thread with the given priority.
    fn spawn_with_priority<R>(
        &self,
        priority: Self::Priority,
        future: impl Future<Output = R> + 'static,
    ) -> Self::Task<R>
    where
        R: 'static;

    /// Enqueues a future for platform idle time, falling back to foreground work when needed.
    fn spawn_when_idle<R>(
        &self,
        timeout: Option<Duration>,
        future: impl Future<Output = R> + 'static,
    ) -> Self::Task<R>
    where
        R: 'static;

    /// Returns the remaining time in the current idle slice, when the platform meters it.
    fn idle_time_remaining(&self) -> Option<Duration>;
}

/// The executor and dispatcher capabilities supplied by a backend runtime.
pub trait RuntimeSpi: 'static {
    /// The dispatcher used by this runtime.
    type Dispatcher: DispatcherSpi + ?Sized;
    /// The background executor used by this runtime.
    type BackgroundExecutor: BackgroundExecutorSpi;
    /// The foreground executor used by this runtime.
    type ForegroundExecutor: ForegroundExecutorSpi;

    /// Borrows the runtime dispatcher.
    fn dispatcher(&self) -> &Arc<Self::Dispatcher>;

    /// Borrows the runtime background executor.
    fn background_executor(&self) -> &Self::BackgroundExecutor;

    /// Borrows the runtime foreground executor.
    fn foreground_executor(&self) -> &Self::ForegroundExecutor;
}

/// The ownership boundary between a GPUI implementation and its host.
///
/// A backend supplies the runtime and application context, while the GPUI API supplies the
/// operations that run against those values. The context bounds make entity storage, context
/// updates, subscriptions, task execution, and window access part of the contract;
/// [`PlatformServicesSpi`] supplies the application-facing platform services.
pub trait BackendSpi: 'static {
    /// The runtime that owns entity-handle lifetimes for [`Self::Context`].
    type Runtime: AppContextRuntime;

    /// The scheduling runtime supplied by the backend.
    type Scheduling: RuntimeSpi;

    /// The context implementation owned by the backend.
    type Context: AppContextSpi
        + AppContextRead
        + AppContextUpdate
        + AppContextObserve
        + AppContextSpawn
        + AppContextWindow;

    /// The platform services injected into the application context.
    type Platform: PlatformServicesSpi;

    /// Borrows the runtime that owns the context's entity handles.
    fn runtime(&self) -> &Self::Runtime;

    /// Borrows the backend-owned scheduling runtime.
    fn scheduling(&self) -> &Self::Scheduling;

    /// Borrows the backend-owned application context.
    fn context(&self) -> &Self::Context;

    /// Mutably borrows the backend-owned application context.
    fn context_mut(&mut self) -> &mut Self::Context;

    /// Borrows the entity storage exposed by the application context.
    fn entity_storage(&self) -> &dyn EntityStorageSpi {
        self.context().entity_storage()
    }

    /// Borrows the injected platform services.
    fn platform(&self) -> &Self::Platform;
}
