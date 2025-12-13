use crate::Builder;
use crate::block_on::block_on;

use core::future::Future;

#[derive(Default)]
pub struct Runtime {}

impl Runtime {
    pub fn block_on<F: Future>(&self, fut: F) -> F::Output {
        block_on(fut)
    }

    pub fn builder() -> Builder {
        Builder::default()
    }
}
