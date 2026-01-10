use crate::reactor::{Reactor, ReactorHandle};
use crate::runtime::Executor;
use crate::runtime::workstealing::Injector;

use std::sync::Arc;

pub struct Runtime {
    injector: Arc<Injector>,
    reactor: ReactorHandle,
}

impl Runtime {
    pub(crate) fn new() -> Self {
        let injector = Arc::new(Injector::new());
        let reactor = Reactor::new();

        let executor = Executor::new(injector.clone(), reactor.clone());
        executor.start();

        Self { injector, reactor }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.injector.shutdown();
    }
}
