use crate::{BackgroundExecutorSpi, DispatcherSpi, ForegroundExecutorSpi, Task};
use futures::channel::mpsc;
use futures::prelude::*;
use scheduler::Instant;
use std::{future::Future, marker::PhantomData, mem, pin::Pin, rc::Rc, sync::Arc, time::Duration};

/// Keeps an operating system activity alive until dropped.
pub struct ActivityGuard {
    release: Option<Box<dyn FnOnce() + Send>>,
}

pub use scheduler::{LocalExecutor as SchedulerLocalExecutor, Priority};

impl ActivityGuard {
    /// Runs `release` when the guard is dropped.
    pub fn new(release: impl FnOnce() + Send + 'static) -> Self {
        Self {
            release: Some(Box::new(release)),
        }
    }

    /// A guard for platforms without a corresponding activity.
    pub fn noop() -> Self {
        Self::new(|| {})
    }
}

impl Drop for ActivityGuard {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            release();
        }
    }
}

/// Backend-owned operations used by the shared [`BackgroundExecutor`] identity.
pub trait BackgroundExecutorBackend: Clone + 'static {
    /// The dispatcher associated with this executor.
    type Dispatcher: DispatcherSpi + ?Sized;

    /// Constructs the backend runtime for a dispatcher.
    fn new(dispatcher: Arc<Self::Dispatcher>) -> Self;

    /// Returns the scheduler executor used by this backend.
    fn scheduler_executor(&self) -> scheduler::BackgroundExecutor;

    /// Prevents platform activity throttling while the returned guard is held.
    fn prevent_app_nap(&self, reason: &str) -> ActivityGuard;

    /// Spawns a closure on a fresh dedicated scheduler session.
    fn spawn_dedicated<F, Fut>(&self, f: F) -> Task<Fut::Output>
    where
        F: FnOnce(SchedulerLocalExecutor) -> Fut + Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send + Sync + 'static;

    /// Enqueues a future on a background thread.
    fn spawn<R>(&self, future: impl Future<Output = R> + Send + 'static) -> Task<R>
    where
        R: Send + 'static;

    /// Enqueues a future on a background thread with the given priority.
    fn spawn_with_priority<R>(
        &self,
        priority: Priority,
        future: impl Future<Output = R> + Send + 'static,
    ) -> Task<R>
    where
        R: Send + 'static;

    /// Returns a task that completes after the given duration.
    fn timer(&self, duration: Duration) -> Task<()>;

    /// Returns the executor's current time.
    fn now(&self) -> Instant;

    /// Returns whether the current thread is the main thread.
    fn is_main_thread(&self) -> bool;

    /// Returns the number of CPUs available to this executor.
    fn num_cpus(&self) -> usize;

    /// The future returned by [`Self::simulate_random_delay`].
    #[cfg(any(test, feature = "test-support"))]
    type RandomDelay: Future<Output = ()> + 'static;

    /// Runs an arbitrary number of deterministic test tasks.
    #[cfg(any(test, feature = "test-support"))]
    fn simulate_random_delay(&self) -> Self::RandomDelay;

    /// Advances deterministic test time without running tasks.
    #[cfg(any(test, feature = "test-support"))]
    fn advance_clock(&self, duration: Duration);

    /// Runs one deterministic test task.
    #[cfg(any(test, feature = "test-support"))]
    fn tick(&self) -> bool;

    /// Runs deterministic test tasks until the scheduler would park.
    #[cfg(any(test, feature = "test-support"))]
    fn run_until_parked(&self);

    /// Allows deterministic test schedulers to park with outstanding tasks.
    #[cfg(any(test, feature = "test-support"))]
    fn allow_parking(&self);

    /// Sets deterministic test scheduler block-on ticks.
    #[cfg(any(test, feature = "test-support"))]
    fn set_block_on_ticks(&self, range: std::ops::RangeInclusive<usize>);

    /// Forbids deterministic test schedulers from parking with outstanding tasks.
    #[cfg(any(test, feature = "test-support"))]
    fn forbid_parking(&self);

    /// Returns the deterministic test scheduler's random generator.
    #[cfg(any(test, feature = "test-support"))]
    fn rng(&self) -> scheduler::SharedRng;

    /// Overrides the CPU count reported by a deterministic test executor.
    #[cfg(any(test, feature = "test-support"))]
    fn set_num_cpus(&self, count: usize);
}

/// Backend-owned operations used by the shared [`ForegroundExecutor`] identity.
pub trait ForegroundExecutorBackend: Clone + 'static {
    /// The dispatcher associated with this executor.
    type Dispatcher: DispatcherSpi + ?Sized;

    /// Constructs the backend runtime for a dispatcher.
    fn new(dispatcher: Arc<Self::Dispatcher>) -> Self;

    /// Returns the scheduler executor used by this backend.
    fn scheduler_executor(&self) -> SchedulerLocalExecutor;

    /// Enqueues a future on the foreground thread.
    fn spawn<R>(&self, future: impl Future<Output = R> + 'static) -> Task<R>
    where
        R: 'static;

    /// Enqueues a future on the foreground thread with the given priority.
    fn spawn_with_priority<R>(
        &self,
        priority: Priority,
        future: impl Future<Output = R> + 'static,
    ) -> Task<R>
    where
        R: 'static;

    /// Enqueues a future for platform idle time.
    fn spawn_when_idle<R>(
        &self,
        timeout: Option<Duration>,
        future: impl Future<Output = R> + 'static,
    ) -> Task<R>
    where
        R: 'static;

    /// Returns the remaining time in the current idle slice.
    fn idle_time_remaining(&self) -> Option<Duration>;
}

/// A pointer to the executor that is currently running, for spawning background tasks.
pub struct BackgroundExecutor<B: BackgroundExecutorBackend> {
    backend: B,
    dispatcher: Arc<B::Dispatcher>,
}

impl<B: BackgroundExecutorBackend> Clone for BackgroundExecutor<B> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            dispatcher: self.dispatcher.clone(),
        }
    }
}

impl<B: BackgroundExecutorBackend> BackgroundExecutor<B> {
    /// Creates a new background executor from the given platform dispatcher.
    pub fn new(dispatcher: Arc<B::Dispatcher>) -> Self {
        Self {
            backend: B::new(dispatcher.clone()),
            dispatcher,
        }
    }

    /// Returns the underlying scheduler background executor.
    pub fn scheduler_executor(&self) -> scheduler::BackgroundExecutor {
        self.backend.scheduler_executor()
    }

    /// Prevents App Nap-style throttling while the returned guard is held.
    pub fn prevent_app_nap(&self, reason: &str) -> ActivityGuard {
        self.backend.prevent_app_nap(reason)
    }

    /// Spawns a closure on a fresh session pinned to its own local scheduler executor.
    #[track_caller]
    pub fn spawn_dedicated<F, Fut>(&self, f: F) -> Task<Fut::Output>
    where
        F: FnOnce(SchedulerLocalExecutor) -> Fut + Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send + Sync + 'static,
    {
        self.backend.spawn_dedicated(f)
    }

    /// Enqueues the given future to be run to completion on a background thread.
    #[track_caller]
    pub fn spawn<R>(&self, future: impl Future<Output = R> + Send + 'static) -> Task<R>
    where
        R: Send + 'static,
    {
        self.backend.spawn(future)
    }

    /// Enqueues the given future on a background thread with the given priority.
    #[track_caller]
    pub fn spawn_with_priority<R>(
        &self,
        priority: Priority,
        future: impl Future<Output = R> + Send + 'static,
    ) -> Task<R>
    where
        R: Send + 'static,
    {
        self.backend.spawn_with_priority(priority, future)
    }

    /// Runs background tasks that may borrow from their environment and waits for all of them.
    #[cfg(not(target_family = "wasm"))]
    pub async fn scoped<'scope, F>(&self, scheduler: F)
    where
        F: FnOnce(&mut Scope<'scope, B>),
    {
        self.scoped_priority(Priority::default(), scheduler).await;
    }

    /// Runs prioritized background tasks that may borrow from their environment and waits for all of them.
    #[cfg(not(target_family = "wasm"))]
    pub async fn scoped_priority<'scope, F>(&self, priority: Priority, scheduler: F)
    where
        F: FnOnce(&mut Scope<'scope, B>),
    {
        let mut scope = Scope::new(self.clone(), priority);
        scheduler(&mut scope);
        let spawned = mem::take(&mut scope.futures)
            .into_iter()
            .map(|future| self.spawn_with_priority(scope.priority, future))
            .collect::<Vec<_>>();
        for task in spawned {
            task.await;
        }
    }

    /// Returns the current time used by this executor.
    pub fn now(&self) -> Instant {
        self.backend.now()
    }

    /// Returns a task that completes after the given duration.
    #[track_caller]
    pub fn timer(&self, duration: Duration) -> Task<()> {
        self.backend.timer(duration)
    }

    /// In tests, runs an arbitrary number of tasks determined by the test seed.
    #[cfg(any(test, feature = "test-support"))]
    pub fn simulate_random_delay(&self) -> impl Future<Output = ()> + use<B> {
        self.backend.simulate_random_delay()
    }

    /// In tests, moves time forward without running tasks.
    #[cfg(any(test, feature = "test-support"))]
    pub fn advance_clock(&self, duration: Duration) {
        self.backend.advance_clock(duration)
    }

    /// In tests, runs one task.
    #[cfg(any(test, feature = "test-support"))]
    pub fn tick(&self) -> bool {
        self.backend.tick()
    }

    /// In tests, runs tasks until the scheduler would park.
    #[cfg(any(test, feature = "test-support"))]
    pub fn run_until_parked(&self) {
        self.backend.run_until_parked()
    }

    /// In tests, permits parking with outstanding tasks.
    #[cfg(any(test, feature = "test-support"))]
    pub fn allow_parking(&self) {
        self.backend.allow_parking()
    }

    /// Sets the range of ticks to run before timing out in block-on tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_block_on_ticks(&self, range: std::ops::RangeInclusive<usize>) {
        self.backend.set_block_on_ticks(range)
    }

    /// Undoes the effect of [`Self::allow_parking`].
    #[cfg(any(test, feature = "test-support"))]
    pub fn forbid_parking(&self) {
        self.backend.forbid_parking()
    }

    /// In tests, returns the random generator used by the dispatcher.
    #[cfg(any(test, feature = "test-support"))]
    pub fn rng(&self) -> scheduler::SharedRng {
        self.backend.rng()
    }

    /// How many CPUs are available to the dispatcher.
    pub fn num_cpus(&self) -> usize {
        self.backend.num_cpus()
    }

    /// Overrides the number of CPUs reported by this executor in tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_num_cpus(&self, count: usize) {
        self.backend.set_num_cpus(count)
    }

    /// Whether the current thread is the main thread.
    pub fn is_main_thread(&self) -> bool {
        self.backend.is_main_thread()
    }

    /// Returns the dispatcher associated with this executor.
    #[doc(hidden)]
    pub fn dispatcher(&self) -> &Arc<B::Dispatcher> {
        &self.dispatcher
    }
}

/// A pointer to the executor that is currently running, for spawning tasks on the main thread.
pub struct ForegroundExecutor<B: ForegroundExecutorBackend> {
    backend: B,
    dispatcher: Arc<B::Dispatcher>,
    not_send: PhantomData<Rc<()>>,
}

impl<B: ForegroundExecutorBackend> Clone for ForegroundExecutor<B> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            dispatcher: self.dispatcher.clone(),
            not_send: PhantomData,
        }
    }
}

impl<B: ForegroundExecutorBackend> ForegroundExecutor<B> {
    /// Creates a new foreground executor from the given platform dispatcher.
    pub fn new(dispatcher: Arc<B::Dispatcher>) -> Self {
        Self {
            backend: B::new(dispatcher.clone()),
            dispatcher,
            not_send: PhantomData,
        }
    }

    /// Enqueues the given task to run on the main thread.
    #[track_caller]
    pub fn spawn<R>(&self, future: impl Future<Output = R> + 'static) -> Task<R>
    where
        R: 'static,
    {
        self.backend.spawn(future)
    }

    /// Enqueues the given task to run on the main thread with the given priority.
    #[track_caller]
    pub fn spawn_with_priority<R>(
        &self,
        priority: Priority,
        future: impl Future<Output = R> + 'static,
    ) -> Task<R>
    where
        R: 'static,
    {
        self.backend.spawn_with_priority(priority, future)
    }

    /// Enqueues the given future to run during platform idle time.
    #[track_caller]
    pub fn spawn_when_idle<R>(
        &self,
        timeout: Option<Duration>,
        future: impl Future<Output = R> + 'static,
    ) -> Task<R>
    where
        R: 'static,
    {
        self.backend.spawn_when_idle(timeout, future)
    }

    /// Returns the remaining time in the current idle slice.
    pub fn idle_time_remaining(&self) -> Option<Duration> {
        self.backend.idle_time_remaining()
    }

    /// Used by the test harness to run an async test synchronously.
    #[cfg(all(not(target_family = "wasm"), any(test, feature = "test-support")))]
    #[track_caller]
    pub fn block_test<R>(&self, future: impl Future<Output = R>) -> R {
        use std::cell::Cell;

        let scheduler_executor = self.scheduler_executor();
        let scheduler = scheduler_executor.scheduler();
        let output = Cell::new(None);
        let future = async {
            output.set(Some(future.await));
        };
        let mut future = std::pin::pin!(future);
        scheduler.block(None, future.as_mut(), None);
        output.take().expect("block_test future did not complete")
    }

    /// Blocks the current thread until the given future resolves.
    #[cfg(not(target_family = "wasm"))]
    pub fn block_on<R>(&self, future: impl Future<Output = R>) -> R {
        self.scheduler_executor().block_on(future)
    }

    /// Blocks the current thread until the future resolves or the timeout elapses.
    #[cfg(not(target_family = "wasm"))]
    pub fn block_with_timeout<R, Fut: Future<Output = R>>(
        &self,
        duration: Duration,
        future: Fut,
    ) -> Result<R, impl Future<Output = R> + use<B, R, Fut>> {
        self.scheduler_executor()
            .block_with_timeout(duration, future)
    }

    /// Returns the scheduler local executor used by this backend.
    #[doc(hidden)]
    pub fn scheduler_executor(&self) -> SchedulerLocalExecutor {
        self.backend.scheduler_executor()
    }

    /// Returns the dispatcher associated with this executor.
    #[doc(hidden)]
    pub fn dispatcher(&self) -> &Arc<B::Dispatcher> {
        &self.dispatcher
    }
}

/// Scope manages a set of tasks that are enqueued and waited on together.
#[cfg(not(target_family = "wasm"))]
pub struct Scope<'a, B: BackgroundExecutorBackend> {
    executor: BackgroundExecutor<B>,
    priority: Priority,
    futures: Vec<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
    tx: Option<mpsc::Sender<()>>,
    rx: mpsc::Receiver<()>,
    lifetime: PhantomData<&'a ()>,
}

#[cfg(not(target_family = "wasm"))]
impl<'a, B: BackgroundExecutorBackend> Scope<'a, B> {
    fn new(executor: BackgroundExecutor<B>, priority: Priority) -> Self {
        let (tx, rx) = mpsc::channel(1);
        Self {
            executor,
            priority,
            tx: Some(tx),
            rx,
            futures: Default::default(),
            lifetime: PhantomData,
        }
    }

    /// How many CPUs are available to the dispatcher.
    pub fn num_cpus(&self) -> usize {
        self.executor.num_cpus()
    }

    /// Spawns a future into this scope.
    #[track_caller]
    pub fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'a,
    {
        let tx = self.tx.clone().expect("scope sender unavailable");
        let future = unsafe {
            mem::transmute::<
                Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
                Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
            >(Box::pin(async move {
                future.await;
                drop(tx);
            }))
        };
        self.futures.push(future);
    }
}

#[cfg(not(target_family = "wasm"))]
impl<B: BackgroundExecutorBackend> Drop for Scope<'_, B> {
    fn drop(&mut self) {
        self.tx.take().expect("scope sender unavailable");
        let future = async {
            self.rx.next().await;
        };
        let mut future = std::pin::pin!(future);
        self.executor
            .scheduler_executor()
            .scheduler()
            .block(None, future.as_mut(), None);
    }
}

pub use scheduler::{DedicatedExecutor, FallibleTask};

impl<B: BackgroundExecutorBackend> BackgroundExecutorSpi for BackgroundExecutor<B> {
    type Task<T> = Task<T>;
    type Priority = Priority;
    type Instant = Instant;

    fn spawn<R>(&self, future: impl Future<Output = R> + Send + 'static) -> Self::Task<R>
    where
        R: Send + 'static,
    {
        BackgroundExecutor::spawn(self, future)
    }

    fn spawn_with_priority<R>(
        &self,
        priority: Self::Priority,
        future: impl Future<Output = R> + Send + 'static,
    ) -> Self::Task<R>
    where
        R: Send + 'static,
    {
        BackgroundExecutor::spawn_with_priority(self, priority, future)
    }

    fn timer(&self, duration: Duration) -> Self::Task<()> {
        BackgroundExecutor::timer(self, duration)
    }

    fn now(&self) -> Self::Instant {
        BackgroundExecutor::now(self)
    }
}

impl<B: ForegroundExecutorBackend> ForegroundExecutorSpi for ForegroundExecutor<B> {
    type Task<T> = Task<T>;
    type Priority = Priority;

    fn spawn<R>(&self, future: impl Future<Output = R> + 'static) -> Self::Task<R>
    where
        R: 'static,
    {
        ForegroundExecutor::spawn(self, future)
    }

    fn spawn_with_priority<R>(
        &self,
        priority: Self::Priority,
        future: impl Future<Output = R> + 'static,
    ) -> Self::Task<R>
    where
        R: 'static,
    {
        ForegroundExecutor::spawn_with_priority(self, priority, future)
    }

    fn spawn_when_idle<R>(
        &self,
        timeout: Option<Duration>,
        future: impl Future<Output = R> + 'static,
    ) -> Self::Task<R>
    where
        R: 'static,
    {
        ForegroundExecutor::spawn_when_idle(self, timeout, future)
    }

    fn idle_time_remaining(&self) -> Option<Duration> {
        ForegroundExecutor::idle_time_remaining(self)
    }
}
