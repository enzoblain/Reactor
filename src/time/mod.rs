//! Time utilities: async sleep and task timing.
//!
//! This module provides time-related async primitives:
//!
//! - [`sleep`] for non-blocking delays
//! - [`wrapper::Time`] for measuring elapsed time of async tasks
//!
//! # Example: Sleep
//!
//! ```ignore
//! use reactor::time::sleep;
//! use std::time::Duration;
//!
//! async fn wait() {
//!     sleep(Duration::from_secs(1)).await;
//! }
//! ```
//!
//! # Example: Timing a Task
//!
//! ```ignore
//! use reactor::time::{sleep, wrapper::Time};
//! use reactor::Task;
//! use std::time::Duration;
//!
//! async fn timed() {
//!     let handle = Task::spawn(async { sleep(Duration::from_millis(10)).await; });
//!     let (_result, elapsed) = Time::new(handle).await;
//!     println!("Elapsed: {:?}", elapsed);
//! }
//! ```

pub mod sleep;
pub mod wrapper;

pub use sleep::sleep;
