//! Timeout utility for async tasks.
//!
//! This module provides a [`timeout`] combinator to wrap an async task or future with a deadline.
//! If the inner future does not complete before the specified duration, a [`TimeError::TimeOut`] is returned.
//!
//! # Example: Timeout a Future
//!
//! ```ignore
//! use reactor::time::{timeout, sleep};
//! use std::time::Duration;
//!
//! async fn run_with_timeout() {
//!     let result = timeout(Duration::from_millis(100), async {
//!         sleep(Duration::from_millis(50)).await;
//!         "done"
//!     }).await;
//!     assert_eq!(result, Ok("done"));
//! }
//!
//! async fn run_timeout_expires() {
//!     let result = timeout(Duration::from_millis(10), async {
//!         sleep(Duration::from_millis(100)).await;
//!         "late"
//!     }).await;
//!     assert!(result.is_err());
//! }
//! ```
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use crate::{ReactorHandle, runtime::context::current_reactor_io, time::TimeError};

/// Wraps a future with a timeout. If the future does not complete before the duration,
/// returns `Err(TimeError::TimeOut)`.
///
/// # Arguments
/// * `duration` - The maximum duration to wait for the future to complete.
/// * `future` - The future to execute.
///
/// # Returns
/// A future that resolves to `Ok(T)` if the inner future completes in time, or `Err(TimeError::TimeOut)` otherwise.
pub fn timeout<F>(duration: Duration, future: F) -> Timeout<F>
where
    F: Future,
{
    Timeout::new(duration, future)
}

/// Future returned by [`timeout`].
///
/// Polls the inner future until it completes or the timeout expires.
pub struct Timeout<F> {
    /// The wrapped future.
    future: F,

    /// The deadline at which the timeout expires.
    deadline: Instant,
    /// The timeout duration.
    duration: Duration,

    /// A handle to the reactor for timer registration.
    reactor: ReactorHandle,

    /// Whether the timer has been registered with the reactor.
    registered: bool,
}

impl<F> Timeout<F> {
    /// Creates a new [`Timeout`] future.
    ///
    /// # Arguments
    /// * `duration` - The timeout duration.
    /// * `future` - The future to wrap.
    pub(crate) fn new(duration: Duration, future: F) -> Self {
        Timeout {
            future,
            deadline: Instant::now() + duration,
            duration,
            reactor: current_reactor_io(),
            registered: false,
        }
    }
}

impl<F: Future> Future for Timeout<F> {
    type Output = Result<F::Output, TimeError>;

    /// Polls the inner future, returning `Ok` if it completes before the timeout,
    /// or `Err(TimeError::TimeOut)` if the deadline is reached first.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            return Poll::Ready(Err(TimeError::TimeOut));
        }

        let fut = unsafe { self.as_mut().map_unchecked_mut(|s| &mut s.future) };
        if let Poll::Ready(v) = fut.poll(cx) {
            return Poll::Ready(Ok(v));
        }

        if !self.registered {
            let waker = cx.waker().clone();
            self.reactor
                .lock()
                .unwrap()
                .register_timer(self.duration, waker);

            unsafe {
                let this = self.get_unchecked_mut();
                this.registered = true;
            }
        }

        Poll::Pending
    }
}
