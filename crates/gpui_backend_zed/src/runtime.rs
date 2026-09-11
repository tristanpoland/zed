use crate::{BackgroundExecutor, ForegroundExecutor, PlatformDispatcher};
use gpui_types::{BackgroundExecutorSpi, DispatcherSpi, ForegroundExecutorSpi, RuntimeSpi};
use scheduler::Instant;
use std::sync::Arc;

/// The scheduling runtime owned by the Zed GPUI backend.
///
/// This keeps the dispatcher and its executor handles together at the backend boundary while
/// leaving the public executor methods on their existing concrete types.
#[derive(Clone)]
pub struct Runtime {
    dispatcher: Arc<dyn PlatformDispatcher>,
    background_executor: BackgroundExecutor,
    foreground_executor: ForegroundExecutor,
}

impl Runtime {
    /// Creates a runtime backed by the given platform dispatcher.
    pub fn new(dispatcher: Arc<dyn PlatformDispatcher>) -> Self {
        let background_executor = BackgroundExecutor::new(dispatcher.clone());
        let foreground_executor = ForegroundExecutor::new(dispatcher.clone());
        Self {
            dispatcher,
            background_executor,
            foreground_executor,
        }
    }

    /// Groups existing executor handles under their shared dispatcher.
    pub fn from_executors(
        background_executor: BackgroundExecutor,
        foreground_executor: ForegroundExecutor,
    ) -> Self {
        let dispatcher = background_executor.dispatcher().clone();
        Self {
            dispatcher,
            background_executor,
            foreground_executor,
        }
    }
}

impl RuntimeSpi for Runtime {
    type Dispatcher = dyn PlatformDispatcher;
    type BackgroundExecutor = BackgroundExecutor;
    type ForegroundExecutor = ForegroundExecutor;

    fn dispatcher(&self) -> &Arc<Self::Dispatcher> {
        &self.dispatcher
    }

    fn background_executor(&self) -> &Self::BackgroundExecutor {
        &self.background_executor
    }

    fn foreground_executor(&self) -> &Self::ForegroundExecutor {
        &self.foreground_executor
    }
}

const _: () = {
    const fn assert_runtime<T: RuntimeSpi>() {}
    const fn assert_background<
        T: BackgroundExecutorSpi<Priority = scheduler::Priority, Instant = Instant>,
    >() {
    }
    const fn assert_foreground<T: ForegroundExecutorSpi<Priority = scheduler::Priority>>() {}
    const fn assert_dispatcher<T: DispatcherSpi<Priority = scheduler::Priority> + ?Sized>() {}

    assert_runtime::<Runtime>();
    assert_background::<BackgroundExecutor>();
    assert_foreground::<ForegroundExecutor>();
    assert_dispatcher::<dyn PlatformDispatcher>();
};
