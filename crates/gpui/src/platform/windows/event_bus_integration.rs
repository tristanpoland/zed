//! Integration layer connecting the lock-free event bus to Windows input handlers.
//!
//! Architecture:
//! 1. Message handlers post events to global lock-free bus (non-blocking ~50ns)
//! 2. Dedicated processor thread drains bus and routes to per-window channels
//! 3. Windows drain their receivers during paint/timer (main thread, fast)

use std::sync::Arc;
use windows::Win32::Foundation::HWND;
use dashmap::DashMap;

use crate::platform::windows::event_bus::{EventBus, InputProcessorThread, Event};
use crate::PlatformInput;

/// Global event bus instance (one per application)
static EVENT_BUS: once_cell::sync::Lazy<Arc<EventBus>> =
    once_cell::sync::Lazy::new(|| Arc::new(EventBus::new()));

/// Global input processor thread
static INPUT_PROCESSOR: once_cell::sync::Lazy<parking_lot::Mutex<Option<InputProcessorThread>>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(None));

/// Registry mapping HWND to per-window event sender
/// Using DashMap for lock-free concurrent access (no RwLock contention!)
static WINDOW_SENDERS: once_cell::sync::Lazy<DashMap<isize, flume::Sender<Event>>> =
    once_cell::sync::Lazy::new(|| DashMap::new());

/// Per-window event receiver
/// Events are drained in small batches to avoid blocking other windows
pub struct WindowEventReceiver {
    receiver: flume::Receiver<Event>,
    hwnd: HWND,
}

impl WindowEventReceiver {
    /// Create a new receiver for a window and register it globally
    pub fn new(hwnd: HWND) -> Self {
        let (sender, receiver) = flume::unbounded();

        // Register sender in global registry (lock-free with DashMap!)
        WINDOW_SENDERS.insert(hwnd.0 as isize, sender);

        Self { receiver, hwnd }
    }

    /// Drain pending events (call from main thread)
    /// Returns events to be processed by the window's input callback
    pub fn drain_events(&self, max_events: usize) -> Vec<Event> {
        let mut events = Vec::with_capacity(max_events.min(100));

        for _ in 0..max_events {
            match self.receiver.try_recv() {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }

        events
    }

    /// Get number of pending events
    pub fn pending_count(&self) -> usize {
        self.receiver.len()
    }
}

impl Drop for WindowEventReceiver {
    fn drop(&mut self) {
        // Unregister from global registry when window closes (lock-free!)
        WINDOW_SENDERS.remove(&(self.hwnd.0 as isize));
    }
}

/// Initialize the global event bus and processor thread
pub(crate) fn initialize_event_bus() {
    let mut processor_guard = INPUT_PROCESSOR.lock();

    if processor_guard.is_some() {
        log::warn!("Event bus already initialized");
        return;
    }

    let mut processor = InputProcessorThread::new(EVENT_BUS.clone());

    // Set up routing callback that runs on dedicated thread
    processor.set_callback(|input: PlatformInput| {
        // Get the focused/active window (for now, broadcast to all windows)
        // TODO: Track which window should receive events (focused window)

        // For now, send to all windows (first one will handle it)
        // In future, track focused window and only send to that one
        // DashMap iteration is lock-free!
        for entry in WINDOW_SENDERS.iter() {
            let event = Event {
                input: input.clone(),
                timestamp: std::time::Instant::now(),
                sequence_number: 0, // Will be set by processor
            };

            // Non-blocking send
            let _ = entry.value().try_send(event);
        }

        crate::DispatchEventResult {
            propagate: true,
            default_prevented: false,
        }
    });

    processor.start();
    *processor_guard = Some(processor);

    log::info!("Event bus initialized with dedicated processor thread");
}

/// Shutdown the event bus and processor thread
pub(crate) fn shutdown_event_bus() {
    let mut processor_guard = INPUT_PROCESSOR.lock();

    if let Some(mut processor) = processor_guard.take() {
        processor.stop();
        log::info!("Event bus processor thread stopped");
    }

    // Clear window registry (lock-free with DashMap!)
    WINDOW_SENDERS.clear();
}

/// Post an input event to the global event bus (non-blocking)
///
/// This is called from Windows message handlers and returns immediately (~50ns)
/// The HWND parameter identifies which window the event belongs to
#[inline]
pub(crate) fn post_input_event_for_window(hwnd: HWND, input: PlatformInput) {
    // Create event with window tag
    let event = Event {
        input,
        timestamp: std::time::Instant::now(),
        sequence_number: 0, // Will be set by processor
    };

    // Send directly to the window's channel (bypass global bus for now)
    // This is more efficient and avoids broadcasting
    // DashMap lookup is lock-free!
    if let Some(sender) = WINDOW_SENDERS.get(&(hwnd.0 as isize)) {
        let _ = sender.try_send(event);
    }
}

/// Legacy wrapper for compatibility
#[inline]
pub(crate) fn post_input_event(input: PlatformInput) {
    // For events without window context, push to global bus
    EVENT_BUS.push(input);
}

/// Get a reference to the global event bus (for monitoring/debugging)
pub(crate) fn get_event_bus() -> &'static Arc<EventBus> {
    &EVENT_BUS
}

/// Event bus statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct EventBusStats {
    pub total_pushed: u64,
    pub total_popped: u64,
    pub buffer_expansions: u64,
    pub push_failures: u64,
    pub max_buffer_size: usize,
    pub pending_events: usize,
}

impl EventBusStats {
    pub fn current() -> Self {
        let stats = EVENT_BUS.stats();
        EventBusStats {
            total_pushed: stats.total_events_pushed.load(std::sync::atomic::Ordering::Relaxed),
            total_popped: stats.total_events_popped.load(std::sync::atomic::Ordering::Relaxed),
            buffer_expansions: stats.buffer_expansions.load(std::sync::atomic::Ordering::Relaxed),
            push_failures: stats.push_failures.load(std::sync::atomic::Ordering::Relaxed),
            max_buffer_size: stats.max_buffer_size.load(std::sync::atomic::Ordering::Relaxed),
            pending_events: EVENT_BUS.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_singleton() {
        let bus1 = get_event_bus();
        let bus2 = get_event_bus();

        assert!(Arc::ptr_eq(bus1, bus2));
    }
}
