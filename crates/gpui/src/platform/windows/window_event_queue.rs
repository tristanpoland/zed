//! Per-window lock-free event queue for complete window isolation.
//!
//! Each window has its own dedicated event queue with zero shared state.
//! This ensures dragging/interacting in one window never affects others.

use crate::PlatformInput;
use std::time::Instant;

/// Event with metadata
#[derive(Clone, Debug)]
pub struct WindowInputEvent {
    pub input: PlatformInput,
    pub timestamp: Instant,
}

/// Per-window event queue using lock-free channel
///
/// Each window owns its own instance - no global state!
pub struct WindowEventQueue {
    sender: flume::Sender<WindowInputEvent>,
    receiver: flume::Receiver<WindowInputEvent>,
}

impl WindowEventQueue {
    /// Create a new event queue for a window
    pub fn new() -> Self {
        let (sender, receiver) = flume::unbounded();
        Self { sender, receiver }
    }

    /// Post an event to this window's queue (non-blocking, ~50ns)
    #[inline]
    pub fn post(&self, input: PlatformInput) {
        let event = WindowInputEvent {
            input,
            timestamp: Instant::now(),
        };

        // Non-blocking send - flume is lock-free internally
        let _ = self.sender.try_send(event);
    }

    /// Drain pending events in small batches (call from main thread during paint)
    pub fn drain_events(&self, max_events: usize) -> Vec<WindowInputEvent> {
        let mut events = Vec::with_capacity(max_events.min(32));

        for _ in 0..max_events {
            match self.receiver.try_recv() {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }

        events
    }

    /// Get number of pending events
    #[inline]
    pub fn pending_count(&self) -> usize {
        self.receiver.len()
    }

    /// Check if queue is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }

    /// Get a clone of the sender for posting from message handlers
    /// This allows message handlers to post without holding window reference
    pub fn sender(&self) -> WindowEventSender {
        WindowEventSender {
            sender: self.sender.clone(),
        }
    }
}

/// Lightweight sender for posting events to a window's queue
///
/// Message handlers hold this to post events without needing full window access
#[derive(Clone)]
pub struct WindowEventSender {
    sender: flume::Sender<WindowInputEvent>,
}

impl WindowEventSender {
    /// Post an event (non-blocking, ~50ns)
    #[inline]
    pub fn post(&self, input: PlatformInput) {
        let event = WindowInputEvent {
            input,
            timestamp: Instant::now(),
        };

        let _ = self.sender.try_send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Modifiers;

    #[test]
    fn test_queue_isolation() {
        let queue1 = WindowEventQueue::new();
        let queue2 = WindowEventQueue::new();

        // Post to queue1
        queue1.post(PlatformInput::KeyDown(crate::KeyDownEvent {
            keystroke: crate::Keystroke {
                key: "a".into(),
                modifiers: Modifiers::default(),
                ime_key: None,
            },
            is_held: false,
        }));

        // queue2 should be empty
        assert_eq!(queue2.pending_count(), 0);
        assert_eq!(queue1.pending_count(), 1);
    }

    #[test]
    fn test_batch_draining() {
        let queue = WindowEventQueue::new();

        // Post 100 events
        for _ in 0..100 {
            queue.post(PlatformInput::KeyDown(crate::KeyDownEvent {
                keystroke: crate::Keystroke {
                    key: "a".into(),
                    modifiers: Modifiers::default(),
                    ime_key: None,
                },
                is_held: false,
            }));
        }

        // Drain in batches of 10
        let batch1 = queue.drain_events(10);
        assert_eq!(batch1.len(), 10);
        assert_eq!(queue.pending_count(), 90);

        let batch2 = queue.drain_events(10);
        assert_eq!(batch2.len(), 10);
        assert_eq!(queue.pending_count(), 80);
    }
}
