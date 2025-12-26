//! Async runtime that executes futures and manages task scheduling.
//!
//! The runtime coordinates the execution of a main future via [`Runtime::block_on`] and handles
//! spawned background tasks. It uses a task queue and executor to manage concurrent execution.
//!
//! # Main Event Loop
//!
//! The runtime's [`block_on`](Runtime::block_on) method implements the main event loop:
//! 1. Polls the main future
//! 2. Executes all ready tasks from the queue
//! 3. Polls for I/O events using the reactor
//! 4. Blocks on I/O when idle, or continues if more tasks are ready
//!
//! # Usage
//!
//! ```ignore
//! let mut runtime = Runtime::new();
//! let result = runtime.block_on(async {
//!     println!("Hello from async");
//!     42
//! });
//! assert_eq!(result, 42);
//! ```
//!
//! # Context Management
//!
//! The runtime establishes a thread-local context that allows spawned tasks to use
//! [`Task::spawn`] without an explicit runtime reference. This enables patterns similar
//! to `tokio::spawn`.
//!
//! [`Task::spawn`]: crate::task::Task::spawn

use crate::reactor::core::{Reactor, ReactorHandle};
use crate::runtime::{Executor, Features, TaskQueue, enter_context};
use crate::task::Task;

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// Main async runtime for executing futures.
///
/// Provides the core API for running futures to completion and spawning background tasks.
/// The runtime maintains an executor, task queue, and reactor for managing concurrent
/// task execution and I/O events.
///
/// # Example
///
/// ```ignore
/// let mut runtime = Runtime::new();
/// runtime.block_on(async {
///     println!("Hello, async world!");
/// });
/// ```
pub struct Runtime {
    /// The task queue shared with the executor and spawned tasks.
    queue: Arc<TaskQueue>,

    /// The executor responsible for running tasks from the queue.
    executor: Executor,

    /// A handle to the reactor for managing I/O events.
    reactor: ReactorHandle,

    /// Whether I/O operations are enabled for this runtime.
    io_enabled: bool,

    /// Whether filesystem operations are enabled for this runtime.
    fs_enabled: bool,
}

impl Runtime {
    /// Creates a new runtime with an empty task queue and executor.
    ///
    /// Initializes the runtime components needed for executing futures and managing
    /// spawned tasks. The runtime starts with an empty task queue and a fresh reactor.
    ///
    /// # Example
    /// ```ignore
    /// let runtime = Runtime::new();
    /// ```
    pub fn new() -> Self {
        Self::with_features(false, false)
    }

    /// Creates a runtime with the requested feature set.
    pub(crate) fn with_features(io_enabled: bool, fs_enabled: bool) -> Self {
        let io_enabled = io_enabled || fs_enabled; // fs support requires reactor-backed I/O
        let queue = Arc::new(TaskQueue::new());
        let executor = Executor::new(queue.clone());
        let reactor = Arc::new(Mutex::new(Reactor::new()));

        Self {
            queue,
            executor,
            reactor,
            io_enabled,
            fs_enabled,
        }
    }

    /// Spawns a background task to be executed concurrently.
    ///
    /// Creates a new task from the provided future and enqueues it for execution.
    /// The task will be executed when [`block_on`](Self::block_on) drains the task queue.
    ///
    /// # Arguments
    /// * `future` - A future with `Output = ()` that will be executed as a background task
    ///
    /// # Example
    /// ```ignore
    /// runtime.spawn(async {
    ///     println!("Background task");
    /// });
    /// ```
    pub fn spawn<F: Future<Output = ()> + 'static>(&self, future: F) {
        let task = Task::new(future, self.queue.clone());
        self.queue.push(task);
    }

    /// Blocks until the given future completes, processing spawned tasks along the way.
    ///
    /// Runs the provided future to completion, executing any spawned tasks from the queue
    /// when the main future is pending. Returns the output of the completed future.
    ///
    /// This method also establishes a runtime context, allowing tasks spawned within
    /// the future to use the global [`Task::spawn`] function without an explicit runtime reference.
    ///
    /// # Arguments
    /// * `future` - The future to execute and wait for completion
    ///
    /// # Returns
    /// The output value of the completed future
    ///
    /// # Example
    /// ```ignore
    /// let result = runtime.block_on(async { 42 });
    /// assert_eq!(result, 42);
    /// ```
    ///
    /// [`Task::spawn`]: crate::task::Task::spawn
    pub fn block_on<F: Future>(&mut self, future: F) -> F::Output {
        let features = Features {
            io_enabled: self.io_enabled,
            fs_enabled: self.fs_enabled,
        };

        enter_context(self.queue.clone(), self.reactor.clone(), features, || {
            let mut future = Box::pin(future);

            let mut is_notified = false;

            /// Clones the waker's data pointer.
            fn clone_waker(data_ptr: *const ()) -> std::task::RawWaker {
                std::task::RawWaker::new(data_ptr, &VTABLE)
            }

            /// Wakes the task by setting the notification flag.
            fn wake(data_ptr: *const ()) {
                unsafe {
                    *(data_ptr as *mut bool) = true;
                }
            }

            /// Wakes the task by reference (same as wake for this implementation).
            fn wake_by_ref(data_ptr: *const ()) {
                unsafe {
                    *(data_ptr as *mut bool) = true;
                }
            }

            /// Drops the waker (no-op for this implementation).
            fn drop_waker(_: *const ()) {}

            static VTABLE: std::task::RawWakerVTable =
                std::task::RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker);

            let raw_waker =
                std::task::RawWaker::new(&mut is_notified as *mut bool as *const (), &VTABLE);
            let waker = unsafe { std::task::Waker::from_raw(raw_waker) };
            let mut context = Context::from_waker(&waker);

            loop {
                // Try to poll the main future
                if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                    // Main future completed, drain remaining tasks
                    for _ in 0..10 {
                        self.executor.run();

                        if self.queue.is_empty() {
                            break;
                        }

                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }

                    return value;
                }

                // Execute all ready tasks
                self.executor.run();

                // Poll for I/O events and wake ready tasks
                self.reactor.lock().unwrap().poll_events();
                self.reactor.lock().unwrap().wake_ready();

                // If the main future was notified, poll it again immediately
                if is_notified {
                    is_notified = false;
                    continue;
                }

                // If there are tasks ready, process them before blocking
                if !self.queue.is_empty() {
                    continue;
                }

                // Nothing ready, block until an I/O event occurs
                self.reactor.lock().unwrap().wait_for_event();
                self.reactor.lock().unwrap().handle_events();
                self.reactor.lock().unwrap().wake_ready();
            }
        })
    }

    /// Returns a handle to the reactor for use by futures.
    ///
    /// This allows futures to register I/O and timer events with the reactor.
    ///
    /// # Returns
    /// A cloned handle to the runtime's reactor
    pub fn reactor_handle(&self) -> ReactorHandle {
        self.reactor.clone()
    }

    /// Returns whether I/O operations are enabled for this runtime.
    pub fn io_enabled(&self) -> bool {
        self.io_enabled
    }

    /// Returns whether filesystem operations are enabled for this runtime.
    pub fn fs_enabled(&self) -> bool {
        self.fs_enabled
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
