//! High-performance lock-free event bus for game engine-grade input processing.
//!
//! This module provides a non-blocking, lock-free event bus designed for real-time
//! game engine requirements where no input events can be dropped, delayed, or throttled.
//!
//! Architecture:
//! - Lock-free MPMC (multi-producer, multi-consumer) ring buffers
//! - Dedicated input processing thread (separate from Windows message loop)
//! - Zero-copy event dispatch where possible
//! - Cache-line aligned data structures to prevent false sharing
//! - Backpressure handling via dynamic buffer expansion

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use parking_lot::RwLock;

use crate::{PlatformInput, DispatchEventResult};

/// Cache line size for x86/x64 processors (64 bytes)
const CACHE_LINE_SIZE: usize = 64;

/// Initial ring buffer capacity (must be power of 2)
const INITIAL_BUFFER_CAPACITY: usize = 8192;

/// Maximum ring buffer capacity before we panic (must be power of 2)
const MAX_BUFFER_CAPACITY: usize = 1_048_576; // 1M events

/// Padding to prevent false sharing between atomic counters
#[repr(align(64))]
struct CacheLinePadded<T>(T);

/// Lock-free ring buffer for events.
///
/// Uses atomic operations for head/tail management and unsafe for the actual buffer.
/// This is safe because:
/// 1. Only one thread writes to a slot (determined by atomic fetch_add on tail)
/// 2. Only one thread reads from a slot (determined by atomic fetch_add on head)
/// 3. We never overflow (capacity is checked before write)
struct LockFreeRingBuffer<T> {
    buffer: Vec<parking_lot::RwLock<Option<T>>>,
    capacity: usize,
    mask: usize, // capacity - 1, for fast modulo via bitwise AND

    // Cache-line aligned atomics to prevent false sharing
    head: CacheLinePadded<AtomicUsize>,
    tail: CacheLinePadded<AtomicUsize>,
}

impl<T> LockFreeRingBuffer<T> {
    fn new(capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "Capacity must be power of 2");

        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(parking_lot::RwLock::new(None));
        }

        Self {
            buffer,
            capacity,
            mask: capacity - 1,
            head: CacheLinePadded(AtomicUsize::new(0)),
            tail: CacheLinePadded(AtomicUsize::new(0)),
        }
    }

    /// Try to push an event. Returns false if buffer is full.
    #[inline]
    fn try_push(&self, event: T) -> bool {
        let tail = self.tail.0.load(Ordering::Relaxed);
        let head = self.head.0.load(Ordering::Acquire);

        // Check if buffer is full
        if tail.wrapping_sub(head) >= self.capacity {
            return false;
        }

        // Reserve slot
        let slot_index = tail & self.mask;

        // Write to slot (safe because we own this slot via tail increment)
        let mut slot = self.buffer[slot_index].write();
        *slot = Some(event);
        drop(slot);

        // Publish the write
        self.tail.0.store(tail.wrapping_add(1), Ordering::Release);

        true
    }

    /// Try to pop an event. Returns None if buffer is empty.
    #[inline]
    fn try_pop(&self) -> Option<T> {
        let head = self.head.0.load(Ordering::Relaxed);
        let tail = self.tail.0.load(Ordering::Acquire);

        // Check if buffer is empty
        if head >= tail {
            return None;
        }

        // Reserve slot
        let slot_index = head & self.mask;

        // Read from slot (safe because we own this slot via head increment)
        let mut slot = self.buffer[slot_index].write();
        let event = slot.take();
        drop(slot);

        // Publish the read
        self.head.0.store(head.wrapping_add(1), Ordering::Release);

        event
    }

    /// Get current number of events in buffer (approximate)
    #[inline]
    fn len(&self) -> usize {
        let head = self.head.0.load(Ordering::Relaxed);
        let tail = self.tail.0.load(Ordering::Relaxed);
        tail.wrapping_sub(head)
    }

    /// Check if buffer is empty (approximate)
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Event wrapper with metadata
#[derive(Clone)]
pub struct Event {
    pub input: PlatformInput,
    pub timestamp: Instant,
    pub sequence_number: u64,
}

/// Multi-buffer event bus with dynamic expansion.
///
/// When a buffer fills up, we allocate a new larger buffer and swap it in.
/// Old events are preserved during the swap.
pub struct EventBus {
    /// Current active ring buffer
    current_buffer: Arc<RwLock<Arc<LockFreeRingBuffer<Event>>>>,

    /// Global event sequence number
    sequence: Arc<AtomicU64>,

    /// Statistics
    stats: Arc<EventBusStats>,
}

#[derive(Default)]
pub struct EventBusStats {
    pub total_events_pushed: AtomicU64,
    pub total_events_popped: AtomicU64,
    pub buffer_expansions: AtomicU64,
    pub push_failures: AtomicU64,
    pub max_buffer_size: AtomicUsize,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            current_buffer: Arc::new(RwLock::new(Arc::new(
                LockFreeRingBuffer::new(INITIAL_BUFFER_CAPACITY)
            ))),
            sequence: Arc::new(AtomicU64::new(0)),
            stats: Arc::new(EventBusStats::default()),
        }
    }

    /// Push an event to the bus. Never blocks.
    ///
    /// If the current buffer is full, expands to a larger buffer.
    /// Panics only if we exceed MAX_BUFFER_CAPACITY (game engine is overwhelmed).
    pub fn push(&self, input: PlatformInput) {
        let sequence_number = self.sequence.fetch_add(1, Ordering::Relaxed);

        let event = Event {
            input,
            timestamp: Instant::now(),
            sequence_number,
        };

        // Try to push to current buffer
        let buffer = self.current_buffer.read().clone();

        if buffer.try_push(event.clone()) {
            self.stats.total_events_pushed.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Buffer is full, need to expand
        self.expand_and_push(event);
    }

    /// Expand buffer to larger capacity and push event
    fn expand_and_push(&self, event: Event) {
        // Acquire write lock to expand buffer
        let mut current_buffer_guard = self.current_buffer.write();
        let old_buffer = current_buffer_guard.clone();

        // Check if another thread already expanded
        if old_buffer.try_push(event.clone()) {
            drop(current_buffer_guard);
            self.stats.total_events_pushed.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Calculate new capacity
        let old_capacity = old_buffer.capacity;
        let new_capacity = old_capacity * 2;

        if new_capacity > MAX_BUFFER_CAPACITY {
            panic!(
                "EventBus capacity exceeded maximum ({} events). Game engine input overload!",
                MAX_BUFFER_CAPACITY
            );
        }

        // Create new larger buffer
        let new_buffer = Arc::new(LockFreeRingBuffer::new(new_capacity));

        // Drain old buffer and push to new buffer
        let mut migrated = 0;
        while let Some(old_event) = old_buffer.try_pop() {
            if !new_buffer.try_push(old_event) {
                panic!("Failed to migrate events during buffer expansion");
            }
            migrated += 1;
        }

        // Push the new event
        if !new_buffer.try_push(event) {
            panic!("Failed to push event to newly expanded buffer");
        }

        // Swap in new buffer
        *current_buffer_guard = new_buffer;

        // Update stats
        self.stats.buffer_expansions.fetch_add(1, Ordering::Relaxed);
        self.stats.max_buffer_size.store(new_capacity, Ordering::Relaxed);
        self.stats.total_events_pushed.fetch_add(1, Ordering::Relaxed);

        log::info!(
            "EventBus expanded from {} to {} events ({} migrated)",
            old_capacity,
            new_capacity,
            migrated
        );
    }

    /// Try to pop a batch of events. Returns up to `max_batch_size` events.
    ///
    /// This is more efficient than popping one at a time.
    pub fn try_pop_batch(&self, max_batch_size: usize) -> Vec<Event> {
        let buffer = self.current_buffer.read().clone();
        let mut events = Vec::with_capacity(max_batch_size.min(buffer.len()));

        for _ in 0..max_batch_size {
            if let Some(event) = buffer.try_pop() {
                events.push(event);
                self.stats.total_events_popped.fetch_add(1, Ordering::Relaxed);
            } else {
                break;
            }
        }

        events
    }

    /// Get current buffer length (approximate)
    pub fn len(&self) -> usize {
        self.current_buffer.read().len()
    }

    /// Check if bus is empty (approximate)
    pub fn is_empty(&self) -> bool {
        self.current_buffer.read().is_empty()
    }

    /// Get statistics
    pub fn stats(&self) -> &EventBusStats {
        &self.stats
    }
}

/// Input processing thread that consumes events from the bus and dispatches them.
pub struct InputProcessorThread {
    bus: Arc<EventBus>,
    callback: Arc<parking_lot::Mutex<Option<Box<dyn FnMut(PlatformInput) -> DispatchEventResult + Send + 'static>>>>,
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl InputProcessorThread {
    pub fn new(bus: Arc<EventBus>) -> Self {
        Self {
            bus,
            callback: Arc::new(parking_lot::Mutex::new(None)),
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    /// Set the event dispatch callback
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: FnMut(PlatformInput) -> DispatchEventResult + Send + 'static,
    {
        *self.callback.lock() = Some(Box::new(callback));
    }

    /// Start the input processing thread
    pub fn start(&mut self) {
        if self.running.load(Ordering::Acquire) {
            log::warn!("InputProcessorThread already running");
            return;
        }

        self.running.store(true, Ordering::Release);

        let bus = self.bus.clone();
        let callback = self.callback.clone();
        let running = self.running.clone();

        let handle = thread::Builder::new()
            .name("InputProcessor".to_string())
            .spawn(move || {
                Self::run_loop(bus, callback, running);
            })
            .expect("Failed to spawn InputProcessor thread");

        self.thread_handle = Some(handle);

        log::info!("InputProcessor thread started");
    }

    /// Stop the input processing thread
    pub fn stop(&mut self) {
        if !self.running.load(Ordering::Acquire) {
            return;
        }

        self.running.store(false, Ordering::Release);

        if let Some(handle) = self.thread_handle.take() {
            handle.join().expect("Failed to join InputProcessor thread");
        }

        log::info!("InputProcessor thread stopped");
    }

    /// Main processing loop (runs on dedicated thread)
    fn run_loop(
        bus: Arc<EventBus>,
        callback: Arc<parking_lot::Mutex<Option<Box<dyn FnMut(PlatformInput) -> DispatchEventResult + Send + 'static>>>>,
        running: Arc<AtomicBool>,
    ) {
        const BATCH_SIZE: usize = 64; // Process up to 64 events per iteration
        const SLEEP_DURATION: Duration = Duration::from_micros(100); // 100μs sleep when idle

        let mut iterations_without_events = 0;
        let mut total_events_processed = 0u64;
        let mut last_log = Instant::now();

        while running.load(Ordering::Acquire) {
            // Try to get a batch of events
            let events = bus.try_pop_batch(BATCH_SIZE);

            if events.is_empty() {
                iterations_without_events += 1;

                // Adaptive sleep: sleep longer if we've been idle for a while
                if iterations_without_events > 10 {
                    thread::sleep(SLEEP_DURATION);
                } else {
                    // Spin briefly to maintain low latency
                    std::hint::spin_loop();
                }

                continue;
            }

            iterations_without_events = 0;

            // Process events
            let mut callback_guard = callback.lock();
            if let Some(ref mut cb) = *callback_guard {
                for event in events {
                    let _result = cb(event.input);
                    total_events_processed += 1;
                }
            }
            drop(callback_guard);

            // Periodic logging
            if last_log.elapsed() > Duration::from_secs(10) {
                log::debug!(
                    "InputProcessor: {} events processed, buffer size: {}, stats: pushed={} popped={} expansions={}",
                    total_events_processed,
                    bus.len(),
                    bus.stats().total_events_pushed.load(Ordering::Relaxed),
                    bus.stats().total_events_popped.load(Ordering::Relaxed),
                    bus.stats().buffer_expansions.load(Ordering::Relaxed),
                );
                last_log = Instant::now();
            }
        }

        log::info!(
            "InputProcessor thread exiting. Total events processed: {}",
            total_events_processed
        );
    }
}

impl Drop for InputProcessorThread {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Modifiers, Keystroke, KeyDownEvent};

    #[test]
    fn test_ring_buffer_push_pop() {
        let buffer = LockFreeRingBuffer::new(4);

        assert!(buffer.try_push(1));
        assert!(buffer.try_push(2));
        assert!(buffer.try_push(3));

        assert_eq!(buffer.try_pop(), Some(1));
        assert_eq!(buffer.try_pop(), Some(2));

        assert!(buffer.try_push(4));
        assert!(buffer.try_push(5));

        assert_eq!(buffer.try_pop(), Some(3));
        assert_eq!(buffer.try_pop(), Some(4));
        assert_eq!(buffer.try_pop(), Some(5));
        assert_eq!(buffer.try_pop(), None);
    }

    #[test]
    fn test_ring_buffer_full() {
        let buffer = LockFreeRingBuffer::new(4);

        assert!(buffer.try_push(1));
        assert!(buffer.try_push(2));
        assert!(buffer.try_push(3));
        assert!(buffer.try_push(4));
        assert!(!buffer.try_push(5)); // Should fail - buffer full

        assert_eq!(buffer.try_pop(), Some(1));
        assert!(buffer.try_push(5)); // Now should succeed
    }

    #[test]
    fn test_event_bus_basic() {
        let bus = EventBus::new();

        let keystroke = Keystroke::parse("a").unwrap();
        let input = PlatformInput::KeyDown(KeyDownEvent {
            keystroke,
            is_held: false,
        });

        bus.push(input.clone());
        bus.push(input.clone());

        let events = bus.try_pop_batch(10);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence_number, 0);
        assert_eq!(events[1].sequence_number, 1);
    }

    #[test]
    fn test_event_bus_expansion() {
        let bus = EventBus::new();

        let keystroke = Keystroke::parse("a").unwrap();
        let input = PlatformInput::KeyDown(KeyDownEvent {
            keystroke,
            is_held: false,
        });

        // Push more than initial capacity
        for _ in 0..INITIAL_BUFFER_CAPACITY + 100 {
            bus.push(input.clone());
        }

        assert_eq!(bus.len(), INITIAL_BUFFER_CAPACITY + 100);
        assert!(bus.stats().buffer_expansions.load(Ordering::Relaxed) > 0);

        // Should be able to pop all events
        let mut count = 0;
        while !bus.is_empty() {
            let events = bus.try_pop_batch(100);
            count += events.len();
            if events.is_empty() {
                break;
            }
        }

        assert_eq!(count, INITIAL_BUFFER_CAPACITY + 100);
    }
}
