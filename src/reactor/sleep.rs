//! Sleep futures for asynchronous delays.
//!
//! This module provides the [`Sleep`] future and the [`sleep`] function for
//! creating asynchronous delays. These primitives allow tasks to pause execution
//! for a specified duration without blocking the runtime.
//!
//! # Example
//!
//! ```ignore
//! use std::time::Duration;
//! use reactor::sleep;
//!
//! async fn delayed_task() {
//!     println!("Starting...");
//!     sleep(Duration::from_secs(1)).await;
//!     println!("1 second later");
//! }
//! ```

use crate::reactor::core::ReactorHandle;
use crate::runtime::context::current_reactor_io;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// A future that completes after a specified duration using reactor timers.
///
/// This future registers a timer with the reactor and completes when the timer expires.
/// The timer is registered lazily on the first poll.
pub struct Sleep {
    /// The duration to sleep.
    duration: Duration,

    /// A handle to the reactor for timer registration.
    reactor: ReactorHandle,

    /// Whether the timer has been registered with the reactor.
    registered: bool,
}

impl Sleep {
    /// Creates a new sleep future with the current reactor.
    ///
    /// This is the typical way to create a `Sleep` future. It automatically uses
    /// the reactor from the current runtime context.
    ///
    /// # Arguments
    /// * `duration` - How long to sleep
    ///
    /// # Panics
    /// Panics if called outside of a runtime context (i.e., not within `Runtime::block_on`).
    ///
    /// # Example
    /// ```ignore
    /// Sleep::new(Duration::from_secs(1)).await;
    /// ```
    pub fn new(duration: Duration) -> Self {
        Self::new_with_reactor(duration, current_reactor_io())
    }

    /// Creates a new sleep future with a specific reactor handle.
    ///
    /// This is the explicit version that allows passing a specific reactor.
    /// Most users should use [`new`](Self::new) instead, which automatically
    /// uses the current runtime's reactor.
    ///
    /// # Arguments
    /// * `duration` - How long to sleep
    /// * `reactor` - A handle to the reactor for managing timer events
    pub fn new_with_reactor(duration: Duration, reactor: ReactorHandle) -> Self {
        Self {
            duration,
            reactor,
            registered: false,
        }
    }
}

/// Creates a sleep future for the given duration.
///
/// This is a convenience function that creates a [`Sleep`] future. The returned
/// future will complete after the specified duration has elapsed.
///
/// # Arguments
/// * `duration` - How long to sleep
///
/// # Panics
/// Panics if called outside of a runtime context.
///
/// # Example
/// ```ignore
/// use std::time::Duration;
/// use reactor::sleep;
///
/// async fn example() {
///     println!("Sleeping...");
///     sleep(Duration::from_secs(2)).await;
///     println!("Awake!");
/// }
/// ```
pub fn sleep(duration: Duration) -> Sleep {
    Sleep::new(duration)
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Zero-duration sleep completes immediately
        if self.duration.is_zero() {
            return Poll::Ready(());
        }

        // Register the timer on first poll
        if !self.registered {
            let waker = cx.waker().clone();
            self.reactor
                .lock()
                .unwrap()
                .register_timer(self.duration, waker);
            self.registered = true;

            return Poll::Pending;
        }

        // Timer has fired, we're ready
        Poll::Ready(())
    }
}
