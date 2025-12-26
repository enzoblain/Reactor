//! Task wrapper that combines futures with waker integration.
//!
//! A task encapsulates a future and provides mechanisms for polling and awakening
//! when the future is ready to make progress. Supports both direct task execution via
//! the runtime and global task spawning without requiring an explicit runtime reference.

use crate::runtime::driver::ensure_driver;
use crate::runtime::{CURRENT_QUEUE, TaskQueue, enter_context, make_waker};

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Context;
use std::task::{Poll, Waker};

/// A spawned task that wraps a future.
///
/// Contains a boxed future and references to the task queue for re-scheduling
/// when the task is awakened.
pub struct Task {
    future: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send>>>>,
    pub(crate) queue: Arc<TaskQueue>,
    completed: AtomicBool,
    waiters: Mutex<Vec<Waker>>, // join handles waiting for completion
}

impl Task {
    /// Creates a new task wrapping the given future.
    ///
    /// Constructs a new Task by boxing the future and storing it with a reference
    /// to the task queue for re-scheduling upon awakening.
    ///
    /// # Arguments
    /// * `fut` - The future to wrap as a task
    /// * `queue` - The task queue for scheduling this task
    ///
    /// # Returns
    /// An Arc-wrapped Task ready for spawning
    pub(crate) fn new(
        fut: impl Future<Output = ()> + Send + 'static,
        queue: Arc<TaskQueue>,
    ) -> Arc<Self> {
        Arc::new(Task {
            future: Mutex::new(Some(Box::pin(fut))),
            queue,
            completed: AtomicBool::new(false),
            waiters: Mutex::new(Vec::new()),
        })
    }

    /// Polls the task's future once.
    ///
    /// Attempts to make progress on the wrapped future. If the future returns Pending,
    /// it is stored back for later polling. If it returns Ready, the task is complete.
    /// Uses a custom waker to enable task re-scheduling.
    ///
    /// This method also establishes the runtime context, allowing spawned tasks to use
    /// the global `spawn()` function.
    pub fn poll(self: &Arc<Self>) {
        enter_context(self.queue.clone(), || {
            let w = make_waker(self.clone());
            let mut cx = Context::from_waker(&w);

            let mut slot = self.future.lock().unwrap();

            if let Some(mut fut) = slot.take() {
                match fut.as_mut().poll(&mut cx) {
                    std::task::Poll::Pending => {
                        *slot = Some(fut);
                    }
                    std::task::Poll::Ready(()) => {
                        self.completed.store(true, Ordering::SeqCst);
                        // Wake any join waiters
                        let mut ws = self.waiters.lock().unwrap();
                        for w in ws.drain(..) {
                            w.wake();
                        }
                    }
                }
            }
        });
    }

    /// Spawns a task on the current runtime context and returns a JoinHandle.
    ///
    /// Mirrors `tokio::spawn`: returns a handle that can be awaited
    /// to know when the task completes. Must be called from within a
    /// runtime context.
    pub fn spawn<F: Future<Output = ()> + Send + 'static>(fut: F) -> JoinHandle {
        CURRENT_QUEUE.with(|current| {
            let queue = current
                .borrow()
                .as_ref()
                .expect("Task::spawn() called outside of a runtime context")
                .clone();

            // Ensure the background driver is running so tasks and I/O progress
            // even if the main future performs blocking operations.
            ensure_driver(queue.clone());

            let task = Task::new(fut, queue.clone());
            // Enqueue for scheduling by the driver's executor
            queue.push(task.clone());
            JoinHandle { task }
        })
    }
}

/// A future that resolves when the associated task completes.
pub struct JoinHandle {
    task: Arc<Task>,
}

impl Future for JoinHandle {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.task.completed.load(Ordering::SeqCst) {
            return Poll::Ready(());
        }
        // Register waker to be notified when task completes
        let mut ws = self.task.waiters.lock().unwrap();
        ws.push(cx.waker().clone());
        Poll::Pending
    }
}

/// A helper to collect multiple `JoinHandle`s and await all of them at once.
/// Hides the explicit loop over individual awaits.
pub struct JoinSet {
    handles: Vec<JoinHandle>,
}

impl JoinSet {
    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    pub fn push(&mut self, h: JoinHandle) {
        self.handles.push(h);
    }

    pub async fn await_all(&mut self) {
        // Await all handles, draining to free memory progressively
        for h in self.handles.drain(..) {
            h.await;
        }
    }
}

impl Default for JoinSet {
    fn default() -> Self {
        Self::new()
    }
}
