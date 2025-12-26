//! Runtime subsystem modules.
//!
//! This module provides the core runtime infrastructure including:
//! - [`core`]: The main [`Runtime`] implementation
//! - [`executor`]: Task execution engine
//! - [`queue`]: Thread-safe task queue
//! - [`waker`]: Task waker implementation
//! - [`context`]: Thread-local runtime context
//! - [`yield_now`]: Cooperative task yielding
//!
//! # Architecture
//!
//! The runtime consists of several interconnected components:
//! - **Runtime**: Main event loop that coordinates everything
//! - **Executor**: Drains the task queue and polls tasks
//! - **TaskQueue**: Thread-safe FIFO queue for ready tasks
//! - **Waker**: Implements the waking protocol to re-queue tasks
//! - **Context**: Thread-local storage for the current runtime
//!
//! [`Runtime`]: core::Runtime

pub(crate) mod context;
mod core;
pub(crate) mod executor;
pub(crate) mod queue;
pub(crate) mod waker;
pub mod yield_now;

pub(crate) use context::{CURRENT_QUEUE, Features, enter_context};
pub use core::Runtime;
pub(crate) use executor::Executor;
pub(crate) use queue::TaskQueue;
pub(crate) use waker::make_waker;
