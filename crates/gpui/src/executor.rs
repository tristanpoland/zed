use crate::{App, PlatformDispatcher, PlatformScheduler};
use futures::prelude::*;
use gpui_types::{ActivityGuard, BackgroundExecutorBackend, ForegroundExecutorBackend};
use gpui_util::{TryFutureExt, TryFutureExtBacktrace};
#[cfg(any(test, feature = "test-support"))]
use scheduler::Yield;
use scheduler::{Instant, Scheduler};
use std::{future::Future, sync::Arc, time::Duration};

#[doc(hidden)]
#[derive(Clone)]
pub struct BackgroundExecutorBackendImpl {
    inner: scheduler::BackgroundExecutor,
    dispatcher: Arc<dyn PlatformDispatcher>,
}

impl BackgroundExecutorBackend for BackgroundExecutorBackendImpl {
    type Dispatcher = dyn PlatformDispatcher;

    fn new(dispatcher: Arc<Self::Dispatcher>) -> Self {
        #[cfg(any(test, feature = "test-support"))]
        let scheduler: Arc<dyn Scheduler> = if let Some(test_dispatcher) = dispatcher.as_test() {
            test_dispatcher.scheduler().clone()
        } else {
            Arc::new(PlatformScheduler::new(dispatcher.clone()))
        };

        #[cfg(not(any(test, feature = "test-support")))]
        let scheduler: Arc<dyn Scheduler> = Arc::new(PlatformScheduler::new(dispatcher.clone()));

        Self {
            inner: scheduler::BackgroundExecutor::new(scheduler),
            dispatcher,
        }
    }

    fn scheduler_executor(&self) -> scheduler::BackgroundExecutor {
        self.inner.clone()
    }

    fn prevent_app_nap(&self, reason: &str) -> ActivityGuard {
        self.dispatcher.prevent_app_nap(reason)
    }

    fn spawn_dedicated<F, Fut>(&self, f: F) -> Task<Fut::Output>
    where
        F: FnOnce(scheduler::LocalExecutor) -> Fut + Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send + Sync + 'static,
    {
        self.inner.spawn_dedicated(f).into()
    }

    fn spawn<R>(&self, future: impl Future<Output = R> + Send + 'static) -> Task<R>
    where
        R: Send + 'static,
    {
        self.inner.spawn(future).into()
    }

    fn spawn_with_priority<R>(
        &self,
        priority: scheduler::Priority,
        future: impl Future<Output = R> + Send + 'static,
    ) -> Task<R>
    where
        R: Send + 'static,
    {
        if priority == scheduler::Priority::RealtimeAudio {
            self.inner.spawn_realtime(future).into()
        } else {
            self.inner.spawn_with_priority(priority, future).into()
        }
    }

    fn timer(&self, duration: Duration) -> Task<()> {
        if duration.is_zero() {
            Task::ready(())
        } else {
            self.inner
                .spawn(self.inner.scheduler().timer(duration))
                .into()
        }
    }

    fn now(&self) -> Instant {
        self.inner.scheduler().clock().now()
    }

    fn is_main_thread(&self) -> bool {
        self.dispatcher.is_main_thread()
    }

    fn num_cpus(&self) -> usize {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(test) = self.dispatcher.as_test() {
            return test.num_cpus_override().unwrap_or(4);
        }
        num_cpus::get()
    }

    #[cfg(any(test, feature = "test-support"))]
    type RandomDelay = Yield;

    #[cfg(any(test, feature = "test-support"))]
    fn simulate_random_delay(&self) -> Self::RandomDelay {
        self.dispatcher.as_test().unwrap().simulate_random_delay()
    }

    #[cfg(any(test, feature = "test-support"))]
    fn advance_clock(&self, duration: Duration) {
        self.dispatcher.as_test().unwrap().advance_clock(duration)
    }

    #[cfg(any(test, feature = "test-support"))]
    fn tick(&self) -> bool {
        self.dispatcher.as_test().unwrap().scheduler().tick()
    }

    #[cfg(any(test, feature = "test-support"))]
    fn run_until_parked(&self) {
        self.dispatcher.as_test().unwrap().scheduler().run()
    }

    #[cfg(any(test, feature = "test-support"))]
    fn allow_parking(&self) {
        self.dispatcher
            .as_test()
            .unwrap()
            .scheduler()
            .allow_parking();
        if std::env::var("GPUI_RUN_UNTIL_PARKED_LOG").ok().as_deref() == Some("1") {
            log::warn!("[gpui::executor] allow_parking: enabled");
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    fn set_block_on_ticks(&self, range: std::ops::RangeInclusive<usize>) {
        self.dispatcher
            .as_test()
            .unwrap()
            .scheduler()
            .set_timeout_ticks(range);
    }

    #[cfg(any(test, feature = "test-support"))]
    fn forbid_parking(&self) {
        self.dispatcher
            .as_test()
            .unwrap()
            .scheduler()
            .forbid_parking()
    }

    #[cfg(any(test, feature = "test-support"))]
    fn rng(&self) -> scheduler::SharedRng {
        self.dispatcher.as_test().unwrap().scheduler().rng()
    }

    #[cfg(any(test, feature = "test-support"))]
    fn set_num_cpus(&self, count: usize) {
        self.dispatcher
            .as_test()
            .expect("set_num_cpus can only be called on a test executor")
            .set_num_cpus(count);
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct ForegroundExecutorBackendImpl {
    inner: scheduler::LocalExecutor,
    dispatcher: Arc<dyn PlatformDispatcher>,
    #[cfg(feature = "profiler")]
    foreground_runnables: Option<crate::profiler::journal::ForegroundRunnableCounter>,
}

impl ForegroundExecutorBackend for ForegroundExecutorBackendImpl {
    type Dispatcher = dyn PlatformDispatcher;

    fn new(dispatcher: Arc<Self::Dispatcher>) -> Self {
        #[cfg(any(test, feature = "test-support"))]
        let (scheduler, session_id): (Arc<dyn Scheduler>, _) =
            if let Some(test_dispatcher) = dispatcher.as_test() {
                (
                    test_dispatcher.scheduler().clone(),
                    test_dispatcher.session_id(),
                )
            } else {
                let platform_scheduler = Arc::new(PlatformScheduler::new(dispatcher.clone()));
                return Self {
                    inner: platform_scheduler.foreground_executor(),
                    dispatcher,
                    #[cfg(feature = "profiler")]
                    foreground_runnables: Some(platform_scheduler.foreground_runnable_counter()),
                };
            };

        #[cfg(not(any(test, feature = "test-support")))]
        let platform_scheduler = Arc::new(PlatformScheduler::new(dispatcher.clone()));
        #[cfg(not(any(test, feature = "test-support")))]
        let inner = platform_scheduler.foreground_executor();
        #[cfg(all(not(any(test, feature = "test-support")), feature = "profiler"))]
        let foreground_runnables = Some(platform_scheduler.foreground_runnable_counter());

        #[cfg(any(test, feature = "test-support"))]
        let inner = {
            let scheduler_for_dispatch = Arc::downgrade(&scheduler);
            scheduler::LocalExecutor::new(session_id, scheduler, move |runnable| {
                if let Some(scheduler) = scheduler_for_dispatch.upgrade() {
                    scheduler.schedule_local(session_id, runnable);
                }
            })
        };

        #[cfg(all(any(test, feature = "test-support"), feature = "profiler"))]
        let foreground_runnables = None;

        Self {
            inner,
            dispatcher,
            #[cfg(feature = "profiler")]
            foreground_runnables,
        }
    }

    fn scheduler_executor(&self) -> scheduler::LocalExecutor {
        self.inner.clone()
    }

    fn spawn<R>(&self, future: impl Future<Output = R> + 'static) -> Task<R>
    where
        R: 'static,
    {
        self.inner.spawn(future.boxed_local()).into()
    }

    fn spawn_with_priority<R>(
        &self,
        _priority: scheduler::Priority,
        future: impl Future<Output = R> + 'static,
    ) -> Task<R>
    where
        R: 'static,
    {
        self.inner.spawn(future).into()
    }

    fn spawn_when_idle<R>(
        &self,
        timeout: Option<Duration>,
        future: impl Future<Output = R> + 'static,
    ) -> Task<R>
    where
        R: 'static,
    {
        let dispatcher = self.dispatcher.clone();
        #[cfg(feature = "profiler")]
        let foreground_runnables = self.foreground_runnables.clone();
        self.inner
            .spawn_with_dispatch(future.boxed_local(), move |runnable| {
                #[cfg(feature = "profiler")]
                if let Some(foreground_runnables) = &foreground_runnables {
                    foreground_runnables.queued();
                }
                gpui_types::DispatcherSpi::dispatch_on_main_thread_when_idle(
                    dispatcher.as_ref(),
                    runnable,
                    scheduler::Priority::Low,
                    timeout,
                );
            })
            .into()
    }

    fn idle_time_remaining(&self) -> Option<Duration> {
        self.dispatcher.idle_time_remaining()
    }
}

/// The shared background executor identity backed by GPUI's platform scheduler.
pub type BackgroundExecutor = gpui_types::BackgroundExecutor<BackgroundExecutorBackendImpl>;
/// The shared foreground executor identity backed by GPUI's platform scheduler.
pub type ForegroundExecutor = gpui_types::ForegroundExecutor<ForegroundExecutorBackendImpl>;
#[cfg(not(target_family = "wasm"))]
/// A group of background tasks joined by the shared executor identity.
pub type Scope<'a> = gpui_types::Scope<'a, BackgroundExecutorBackendImpl>;

pub use gpui_types::{DedicatedExecutor, FallibleTask, Priority, SchedulerLocalExecutor};

/// Extension trait for `Task<Result<T, E>>` that adds `detach_and_log_err` with an `&App` context.
pub trait TaskExt<T, E> {
    /// Run the task to completion in the background and log any errors that occur.
    fn detach_and_log_err(self, cx: &App);
    /// Like [`Self::detach_and_log_err`], but uses `{:?}` formatting on failure.
    fn detach_and_log_err_with_backtrace(self, cx: &App);
}

impl<T, E> TaskExt<T, E> for Task<Result<T, E>>
where
    T: 'static,
    E: 'static + std::fmt::Display + std::fmt::Debug,
{
    #[track_caller]
    fn detach_and_log_err(self, cx: &App) {
        let location = core::panic::Location::caller();
        cx.foreground_executor()
            .spawn(self.log_tracked_err(*location))
            .detach();
    }

    #[track_caller]
    fn detach_and_log_err_with_backtrace(self, cx: &App) {
        let location = *core::panic::Location::caller();
        cx.foreground_executor()
            .spawn(self.log_tracked_err_with_backtrace(location))
            .detach();
    }
}

pub use gpui_types::Task;

#[cfg(test)]
mod test {
    use super::*;
    use crate::{App, TestDispatcher, TestPlatform};
    use std::{cell::RefCell, rc::Rc};

    fn create_test_app() -> (
        TestDispatcher,
        BackgroundExecutor,
        std::rc::Rc<crate::AppCell>,
    ) {
        let dispatcher = TestDispatcher::new(0);
        let arc_dispatcher = Arc::new(dispatcher.clone());
        let background_executor = BackgroundExecutor::new(arc_dispatcher.clone());
        let foreground_executor = ForegroundExecutor::new(arc_dispatcher);
        let platform = TestPlatform::new(background_executor.clone(), foreground_executor);
        let asset_source = Arc::new(());
        let http_client = http_client::FakeHttpClient::with_404_response();
        let app = App::new_app(platform, asset_source, http_client);
        (dispatcher, background_executor, app)
    }

    #[test]
    fn sanity_test_tasks_run() {
        let (dispatcher, _background_executor, app) = create_test_app();
        let foreground_executor = app.borrow().foreground_executor.clone();
        let task_ran = Rc::new(RefCell::new(false));
        foreground_executor
            .spawn({
                let task_ran = Rc::clone(&task_ran);
                async move {
                    *task_ran.borrow_mut() = true;
                }
            })
            .detach();
        dispatcher.run_until_parked();
        assert!(
            *task_ran.borrow(),
            "Task should run normally when app is alive"
        );
    }
}
