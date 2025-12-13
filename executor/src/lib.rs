#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod block_on;
mod builder;
mod runtime;

pub use builder::Builder;
pub use runtime::Runtime;

#[cfg(feature = "std")]
pub use executor_macros::*;
