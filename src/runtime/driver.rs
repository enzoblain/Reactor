//! Background runtime driver to pump tasks and I/O.
//!
//! Some user code may block inside the main future (e.g., joining a std::thread),
//! preventing the single-threaded runtime loop from progressing. To avoid hangs,
//! this driver runs the executor and reactor in a dedicated thread so spawned
//! tasks and I/O readiness continue to be processed.

use crate::reactor::core::{Reactor, set_current_reactor};
use crate::runtime::{Executor, TaskQueue, enter_context};

use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

// Track which queues already have a driver thread.
static STARTED_QUEUES: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();

/// Starts a background driver thread once for the given queue.
///
/// Safe to call multiple times; only the first call will spawn the thread.
pub(crate) fn ensure_driver(queue: Arc<TaskQueue>) {
    let set = STARTED_QUEUES.get_or_init(|| Mutex::new(HashSet::new()));
    let key = Arc::as_ptr(&queue) as usize;

    {
        let mut started = set.lock().unwrap();
        if started.contains(&key) {
            return;
        }
        started.insert(key);
    }

    thread::spawn(move || {
        let mut reactor = Reactor::new();
        let executor = Executor::new(queue.clone());

        // The driver owns its reactor and sets it as current for this thread.
        set_current_reactor(&mut reactor);

        // Use the provided queue as the runtime context within the driver thread.
        enter_context(queue.clone(), || {
            loop {
                // Check if shutdown has been requested
                if queue.is_shutdown() {
                    break;
                }

                // Drain ready tasks
                executor.run();

                // Handle I/O without blocking when possible
                reactor.poll_events();
                reactor.wake_ready();

                // When tasks remain, keep looping without blocking to deliver wakes promptly
                if !queue.is_empty() {
                    continue;
                }

                // Check shutdown again before blocking
                if queue.is_shutdown() {
                    break;
                }

                // When idle, block on I/O with a timeout to periodically check shutdown
                reactor.wait_for_event_with_timeout(100); // 100ms timeout
                reactor.handle_events();
                reactor.wake_ready();
            }
        });
    });
}
