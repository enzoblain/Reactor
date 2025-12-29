//! Waker implementation for task wake-up notifications.
//!
//! Provides task waker objects that notify the executor when a task is ready to continue.
//! Implements the standard Rust task waking protocol using [`RawWaker`] and [`RawWakerVTable`].
//!
//! [`RawWaker`]: std::task::RawWaker
//! [`RawWakerVTable`]: std::task::RawWakerVTable

use crate::task::Task;

use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

/// Custom waker that re-queues generic tasks when awakened.
///
/// Implements the Rust waker protocol to automatically re-enqueue a task
/// when it becomes ready to make further progress. The generic parameter `T` allows
/// the waker to work with any task output type.
///
/// # Type Parameters
///
/// * `T` - The output type of the associated task
pub struct TaskWaker<T> {
    /// The task to wake when notified.
    task: Arc<Task<T>>,
}

impl<T: 'static> TaskWaker<T> {
    /// Creates a new waker for the given generic task.
    ///
    /// # Arguments
    /// * `task` - The task to wake when notified
    ///
    /// # Returns
    /// An Arc-wrapped `TaskWaker<T>`
    pub fn new(task: Arc<Task<T>>) -> Arc<Self> {
        Arc::new(Self { task })
    }

    /// Wakes the task by re-enqueueing it.
    fn wake(self: &Arc<Self>) {
        self.task.queue.push(self.task.clone());
    }

    /// Raw waker clone function for [`RawWakerVTable`].
    ///
    /// [`RawWakerVTable`]: std::task::RawWakerVTable
    fn clone_raw(data_ptr: *const ()) -> RawWaker {
        unsafe {
            let arc = Arc::<TaskWaker<T>>::from_raw(data_ptr as *const TaskWaker<T>);
            let cloned = arc.clone();
            std::mem::forget(arc);

            RawWaker::new(Arc::into_raw(cloned) as *const (), &Self::VTABLE)
        }
    }

    /// Raw waker wake function for [`RawWakerVTable`].
    ///
    /// [`RawWakerVTable`]: std::task::RawWakerVTable
    fn wake_raw(data_ptr: *const ()) {
        unsafe {
            let arc = Arc::<TaskWaker<T>>::from_raw(data_ptr as *const TaskWaker<T>);
            arc.wake();
        }
    }

    /// Raw waker wake-by-reference function for [`RawWakerVTable`].
    ///
    /// [`RawWakerVTable`]: std::task::RawWakerVTable
    fn wake_by_ref_raw(data_ptr: *const ()) {
        unsafe {
            let arc = Arc::<TaskWaker<T>>::from_raw(data_ptr as *const TaskWaker<T>);
            arc.wake();
            let _ = Arc::into_raw(arc);
        }
    }

    /// Raw waker drop function for [`RawWakerVTable`].
    ///
    /// [`RawWakerVTable`]: std::task::RawWakerVTable
    fn drop_raw(data_ptr: *const ()) {
        unsafe {
            Arc::<TaskWaker<T>>::from_raw(data_ptr as *const TaskWaker<T>);
        }
    }

    /// Virtual method table for raw waker operations.
    pub const VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone_raw,
        Self::wake_raw,
        Self::wake_by_ref_raw,
        Self::drop_raw,
    );
}

/// Creates a [`Waker`] from a generic [`Task<T>`] that re-queues on wake.
///
/// Constructs a Waker that implements the standard Rust task waking protocol.
/// When woken, the task is pushed back to the queue for re-execution.
///
/// # Type Parameters
///
/// * `T` - The output type of the associated task
///
/// # Arguments
/// * `task` - The task to create a waker for
///
/// # Returns
/// A `Waker` that will re-queue the task when called
///
/// [`Waker`]: std::task::Waker
/// [`Task`]: crate::task::Task
pub(crate) fn make_waker<T: 'static>(task: Arc<Task<T>>) -> Waker {
    let task_waker = TaskWaker::new(task);
    let raw_waker = RawWaker::new(
        Arc::into_raw(task_waker) as *const (),
        &TaskWaker::<T>::VTABLE,
    );

    unsafe { Waker::from_raw(raw_waker) }
}
