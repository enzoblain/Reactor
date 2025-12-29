//! Time measurement wrapper for asynchronous tasks.
//!
//! This module provides the [`Time`] future, which wraps a [`JoinHandle`] and measures the elapsed
//! duration from creation until the wrapped task completes. This is useful for benchmarking or
//! timing async operations in a composable way.
//!
//! # Example
//!
//! ```ignore
//! use reactor::time::{sleep, wrapper::Time};
//! use reactor::Task;
//! use std::time::Duration;
//!
//! async fn timed_sleep() {
//!     let handle = Task::spawn(async {
//!         sleep(Duration::from_millis(100)).await;
//!     });
//!     let (_result, elapsed) = Time::new(handle).await;
//!     println!("Slept for {:?}", elapsed);
//! }
//! ```

use crate::JoinHandle;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

/// A future that wraps a [`JoinHandle`] and measures elapsed time until completion.
///
/// When awaited, returns a tuple `(T, Duration)` where `T` is the result of the wrapped task
/// and `Duration` is the time elapsed since creation of the [`Time`] wrapper.
pub struct Time<T> {
    /// The instant when the wrapper was created.
    start: Instant,
    /// The handle to the asynchronous task being measured.
    handle: JoinHandle<T>,
}

impl<T> Time<T> {
    /// Creates a new [`Time`] wrapper for the given [`JoinHandle`].
    ///
    /// # Arguments
    /// * `handle` - The join handle of the async task to measure
    ///
    /// # Example
    /// ```ignore
    /// let handle = Task::spawn(async { /* ... */ });
    /// let (_result, elapsed) = Time::new(handle).await;
    /// ```
    pub fn new(handle: JoinHandle<T>) -> Self {
        Self {
            start: Instant::now(),
            handle,
        }
    }
}

impl<T> Future for Time<T> {
    type Output = (T, Duration);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        match Pin::new(&mut this.handle).poll(cx) {
            Poll::Ready(result) => {
                let elapsed = this.start.elapsed();
                Poll::Ready((result, elapsed))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
