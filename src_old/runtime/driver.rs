use crate::reactor::core::{Reactor, ReactorHandle};
use crate::runtime::{Executor, TaskQueue, enter_context};

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};

static STARTED_QUEUES: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();

pub(crate) struct BackgroundDriver {
    reactor: ReactorHandle,
    executor: Executor,
    queue: Arc<TaskQueue>,
}

impl BackgroundDriver {
    fn new(queue: Arc<TaskQueue>) -> Self {
        let reactor = Rc::new(RefCell::new(cadentis::new()));
        let executor = Executor::new(queue.clone());

        Self {
            reactor,
            executor,
            queue,
        }
    }

    fn run(&mut self) {
        enter_context(self.queue.clone(), || {
            loop {
                if self.queue.is_shutdown() {
                    break;
                }

                self.executor.run();

                self.reactor.borrow_mut().poll_events();
                self.reactor.borrow_mut().wake_ready();

                if !self.queue.is_empty() {
                    continue;
                }

                if self.queue.is_shutdown() {
                    break;
                }

                self.reactor.borrow_mut().wait_for_event_with_timeout(100);
                self.reactor.borrow_mut().handle_events();
                self.reactor.borrow_mut().wake_ready();
            }
        });
    }
}
