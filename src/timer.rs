//! Timer and sleep functionality for the async runtime.
//!
//! Provides `sleep()` function to create futures that resolve after a specified duration.
//! Uses a TimerDriver to avoid busy polling: timers are registered once and awakened
//! explicitly when their deadline is reached.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

// Global timer driver (single-threaded, safe to use with RefCell)
thread_local! {
    static TIMER_DRIVER: RefCell<TimerDriver> = RefCell::new(TimerDriver::new());
}

/// Manages registered timers and wakes them when their deadline is reached.
///
/// Stores a list of (deadline, Waker) pairs and provides methods to:
/// - Register a new timer with its deadline and waker
/// - Check and fire all timers whose deadline has passed
pub struct TimerDriver {
    timers: Vec<(Instant, Waker)>,
}

impl TimerDriver {
    /// Creates a new empty timer driver.
    fn new() -> Self {
        Self { timers: Vec::new() }
    }

    /// Registers a new timer with the given deadline and waker.
    ///
    /// # Arguments
    /// * `deadline` - The time when the timer should fire
    /// * `waker` - The waker to call when the deadline is reached
    pub fn register(&mut self, deadline: Instant, waker: Waker) {
        self.timers.push((deadline, waker));
    }

    /// Checks all registered timers and wakes those that have reached their deadline.
    ///
    /// Removes completed timers from the list and calls their wakers.
    /// Returns `true` if there are still pending timers, `false` if all are done.
    pub fn fire_expired(&mut self) -> bool {
        let now = Instant::now();
        self.timers.retain(|(deadline, waker)| {
            if now >= *deadline {
                waker.wake_by_ref();
                false // Remove this timer
            } else {
                true // Keep this timer
            }
        });
        !self.timers.is_empty()
    }

    /// Returns the time remaining until the next timer deadline, if any.
    fn next_remaining(&self) -> Option<std::time::Duration> {
        let now = Instant::now();
        self.timers
            .iter()
            .map(|(dl, _)| {
                if *dl <= now {
                    std::time::Duration::from_millis(0)
                } else {
                    *dl - now
                }
            })
            .min()
    }
}

/// A future that completes after a specified duration.
///
/// Created via the `sleep()` function, this future registers itself with the TimerDriver
/// on the first poll and then yields `Poll::Pending`. It will be awakened via waker.wake()
/// when the deadline is reached, avoiding busy polling.
#[derive(Debug)]
pub struct Sleep {
    deadline: Instant,
    registered: bool,
}

impl Sleep {
    /// Creates a new sleep future with the given duration.
    ///
    /// # Arguments
    /// * `duration` - How long to sleep
    pub fn new(duration: Duration) -> Self {
        Self {
            deadline: Instant::now() + duration,
            registered: false,
        }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Check if deadline has already been reached
        if Instant::now() >= self.deadline {
            return Poll::Ready(());
        }

        // Register with TimerDriver on first poll only
        if !self.registered {
            TIMER_DRIVER.with(|driver| {
                driver
                    .borrow_mut()
                    .register(self.deadline, cx.waker().clone());
            });
            self.registered = true;
        }

        Poll::Pending
    }
}

/// Sleeps for the specified duration.
///
/// Returns a future that completes after the given duration has elapsed.
/// The sleep is implemented via a TimerDriver that wakes the task explicitly
/// when the deadline is reached, avoiding busy polling.
///
/// # Arguments
/// * `duration` - How long to sleep
///
/// # Example
/// ```ignore
/// use reactor::sleep;
/// use std::time::Duration;
///
/// async {
///     sleep(Duration::from_millis(100)).await;
///     println!("Woke up after 100ms");
/// };
/// ```
pub fn sleep(duration: Duration) -> Sleep {
    Sleep::new(duration)
}

/// Triggers all expired timers in the TimerDriver.
///
/// This function is called by the runtime's main loop to check and fire any timers
/// whose deadline has passed. It's part of the runtime integration.
///
/// Returns `true` if there are still pending timers, `false` if all have been fired.
pub(crate) fn process_timers() -> bool {
    TIMER_DRIVER.with(|driver| driver.borrow_mut().fire_expired())
}

/// Returns the remaining duration until the next scheduled timer, if any.
/// Used by the runtime to avoid blocking on I/O when only timers are pending.
pub(crate) fn next_timer_remaining() -> Option<Duration> {
    TIMER_DRIVER.with(|driver| driver.borrow().next_remaining())
}
