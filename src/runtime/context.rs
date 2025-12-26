//! Thread-local runtime context and feature gating for async task spawning and I/O.
//!
//! This module manages the thread-local state required for global task spawning (`Task::spawn`) and
//! for safe, feature-gated access to the runtime's reactor (I/O, timers, filesystem).
//!
//! # Purpose
//!
//! - Provides thread-local storage for the current runtime context (task queue, reactor handle, features).
//! - Ensures that I/O and filesystem operations are only accessible if explicitly enabled in the runtime.
//! - Used internally by async primitives to access the current runtime without explicit handles.
//!
//! # Usage
//!
//! This module is not intended for direct use by most users. It is used by the runtime and async primitives
//! to manage context during `block_on` and task execution. See [`enter_context`] for details.
//!
//! # Example
//!
//! ```ignore
//! use reactor::runtime::context::{enter_context, Features};
//! use std::sync::Arc;
//! use reactor::runtime::queue::TaskQueue;
//! use reactor::reactor::core::ReactorHandle;
//!
//! let queue = Arc::new(TaskQueue::new());
//! let reactor = todo!(); // Replace with a real ReactorHandle in real code
//! let features = Features { io_enabled: true, fs_enabled: false };
//! enter_context(queue, reactor, features, || {
//!     // ... async code ...
//! });
//! ```
//!
//! # Safety
//!
//! This module uses thread-local storage to ensure each thread has its own context.
//! Feature access is checked on every sensitive API call (I/O, FS).

use crate::reactor::core::ReactorHandle;
use crate::runtime::queue::TaskQueue;

use std::cell::RefCell;
use std::sync::Arc;

/// Feature switches for the current runtime context.
///
/// This struct is used internally to gate access to I/O and filesystem APIs.
/// It is set by the runtime when entering a new context (see [`enter_context`]).
///
/// - `io_enabled`: If true, reactor-backed I/O is enabled for this runtime context.
/// - `fs_enabled`: If true, filesystem support is enabled for this runtime context.
///
/// # Example
///
/// ```ignore
/// let features = Features { io_enabled: true, fs_enabled: false };
/// ```
#[derive(Clone, Copy, Debug)]
pub(crate) struct Features {
    /// If true, reactor-backed I/O is enabled for this runtime context.
    pub(crate) io_enabled: bool,

    /// If true, filesystem support is enabled for this runtime context.
    pub(crate) fs_enabled: bool,
}

thread_local! {
    /// Thread-local storage for the current runtime's task queue.
    ///
    /// Set by [`enter_context`] at the start of every `block_on`.
    pub(crate) static CURRENT_QUEUE: RefCell<Option<Arc<TaskQueue>>> = const { RefCell::new(None) };

    /// Thread-local storage for the current runtime's reactor handle.
    ///
    /// Set by [`enter_context`] at the start of every `block_on`.
    pub(crate) static CURRENT_REACTOR: RefCell<Option<ReactorHandle>> = const { RefCell::new(None) };

    /// Thread-local storage for the current runtime's feature set.
    ///
    /// Set by [`enter_context`] at the start of every `block_on`.
    pub(crate) static CURRENT_FEATURES: RefCell<Option<Features>> = const { RefCell::new(None) };
}
/// This enables patterns like [`TcpListener::bind`] without passing a reactor.
///
/// Enters a new async runtime context for the current thread.
///
/// This function is called automatically by the runtime at each `block_on`.
/// It sets the task queue, reactor handle, and feature set in thread-local storage,
/// then executes the provided closure. The previous context is restored on exit.
///
/// # Arguments
/// - `queue`: The task queue to use in this context.
/// - `reactor`: The reactor handle to use.
/// - `features`: The feature set (I/O, FS) to enable.
/// - `function`: Closure to execute within this context.
///
/// # Example
///
/// ```ignore
/// use reactor::runtime::context::{enter_context, Features};
/// use std::sync::Arc;
/// use reactor::runtime::queue::TaskQueue;
/// use reactor::reactor::core::ReactorHandle;
///
/// let queue = Arc::new(TaskQueue::new());
/// let reactor = todo!(); // Replace with a real ReactorHandle in real code
/// let features = Features { io_enabled: true, fs_enabled: false };
/// enter_context(queue, reactor, features, || {
///     // ... async code ...
/// });
/// ```
pub(crate) fn enter_context<F, R>(
    queue: Arc<TaskQueue>,
    reactor: ReactorHandle,
    features: Features,
    function: F,
) -> R
where
    F: FnOnce() -> R,
{
    CURRENT_QUEUE.with(|current_queue| {
        CURRENT_REACTOR.with(|current_reactor| {
            CURRENT_FEATURES.with(|current_features| {
                let previous_queue = current_queue.borrow_mut().replace(queue.clone());
                let previous_reactor = current_reactor.borrow_mut().replace(reactor.clone());
                let previous_features = current_features.borrow_mut().replace(features);

                let result = function();

                *current_queue.borrow_mut() = previous_queue;
                *current_reactor.borrow_mut() = previous_reactor;
                *current_features.borrow_mut() = previous_features;

                result
            })
        })
    })
}

/// Returns the current reactor handle for I/O operations.
///
/// # Panics
/// Panics if the runtime was not built with `.enable_io()`.
pub(crate) fn current_reactor_io() -> ReactorHandle {
    ensure_feature(|f| f.io_enabled, "I/O", "RuntimeBuilder::enable_io()");

    current_reactor_inner()
}

/// Returns the current reactor handle for filesystem operations.
///
/// # Panics
/// Panics if the runtime was not built with `.enable_fs()`.
pub(crate) fn current_reactor_fs() -> ReactorHandle {
    ensure_feature(
        |f| f.fs_enabled,
        "filesystem",
        "RuntimeBuilder::enable_fs()",
    );

    current_reactor_inner()
}

// Checks that a feature is enabled in the current context, panics otherwise.
fn ensure_feature(check: impl Fn(&Features) -> bool, name: &str, hint: &str) {
    CURRENT_FEATURES.with(|features| {
        let enabled = features.borrow().as_ref().map(check).unwrap_or(false);

        if !enabled {
            panic!("{} support not enabled. Use {}.", name, hint);
        }
    })
}

// Returns the current reactor handle, or panics if not in a runtime context.
fn current_reactor_inner() -> ReactorHandle {
    CURRENT_REACTOR.with(|current| {
        current.borrow().clone().expect(
            "No reactor in current context. I/O operations must be called within Runtime::block_on",
        )
    })
}
