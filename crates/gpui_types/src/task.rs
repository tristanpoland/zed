use crate::TaskHandle;
use std::{
    any::Any,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// A task scheduled by a GPUI executor.
///
/// The scheduler owns task execution and cancellation. This wrapper is the shared public handle
/// identity used by GPUI API facades and backend implementations.
#[must_use]
pub struct Task<T>(scheduler::Task<T>);

impl<T> Task<T> {
    /// Creates a task that is ready with the given value.
    pub fn ready(value: T) -> Self {
        Self(scheduler::Task::ready(value))
    }

    /// Creates a task from an `async_task::Task`.
    pub fn from_async_task(task: async_task::Task<T, scheduler::RunnableMeta>) -> Self {
        Self(scheduler::Task::from_async_task(task))
    }

    /// Returns whether this task has completed.
    pub fn is_ready(&self) -> bool {
        self.0.is_ready()
    }

    /// Detaches this task so it runs independently of its handle.
    pub fn detach(self) {
        self.0.detach();
    }

    /// Converts this task into a fallible task that returns `Option<T>`.
    pub fn fallible(self) -> scheduler::FallibleTask<T> {
        self.0.fallible()
    }
}

impl<T> From<scheduler::Task<T>> for Task<T> {
    fn from(task: scheduler::Task<T>) -> Self {
        Self(task)
    }
}

impl<T> std::fmt::Debug for Task<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<T: 'static> Future for Task<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { self.map_unchecked_mut(|task| &mut task.0) }.poll(context)
    }
}

impl Task<Box<dyn Any + Send + Sync>> {
    /// Reinterprets the boxed output as a concrete `T` when the task completes.
    pub fn downcast<T: Send + Sync + 'static>(self) -> Task<T> {
        Task(self.0.downcast())
    }
}

impl<T> TaskHandle<T> for Task<T> {
    fn detach(self) {
        Task::detach(self)
    }
}
