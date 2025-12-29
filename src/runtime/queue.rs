//! Thread-safe task queue for managing ready tasks.
//!
//! Provides a FIFO queue that allows pushing tasks to be executed and popping
//! them for execution by the executor. The queue is thread-safe and can be
//! safely shared across multiple threads.

use crate::task::Runnable;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// A thread-safe, FIFO queue for storing executable tasks of any type.
///
/// Uses a Mutex-wrapped VecDeque to allow safe concurrent access from multiple threads.
/// The queue stores trait objects implementing [`Runnable`], enabling heterogeneous task types
/// (e.g., different generic parameters) to be managed uniformly by the executor.
/// Tasks are pushed when spawned and popped by the executor for execution.
///
/// [`Runnable`]: crate::task::Runnable
pub(crate) struct TaskQueue {
    /// The internal queue protected by a mutex for thread safety.
    pub(crate) queue: Mutex<VecDeque<Arc<dyn Runnable>>>,
}

impl TaskQueue {
    /// Creates a new empty task queue.
    ///
    /// # Returns
    /// A new `TaskQueue` with no tasks
    pub(crate) fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    /// Enqueues a task to be executed.
    ///
    /// Pushes the task to the back of the queue using FIFO ordering.
    /// Accepts any task implementing [`Runnable`], allowing generic and non-generic tasks to coexist.
    ///
    /// # Arguments
    /// * `task` - The task to enqueue (must implement [`Runnable`])
    ///
    /// [`Runnable`]: crate::task::Runnable
    pub(crate) fn push(&self, task: Arc<dyn Runnable>) {
        self.queue.lock().unwrap().push_back(task);
    }

    /// Dequeues and returns the next ready task.
    ///
    /// # Returns
    /// `Some(task)` if a task is available, `None` if the queue is empty. The returned value is a trait object
    /// implementing [`Runnable`], which can be polled by the executor.
    ///
    /// [`Runnable`]: crate::task::Runnable
    pub(crate) fn pop(&self) -> Option<Arc<dyn Runnable>> {
        self.queue.lock().unwrap().pop_front()
    }

    /// Checks if the task queue is empty.
    ///
    /// # Returns
    /// `true` if the queue contains no tasks, `false` otherwise
    pub(crate) fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }
}
