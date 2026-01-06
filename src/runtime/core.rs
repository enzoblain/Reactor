use crate::reactor::core::{Reactor, ReactorHandle};
use crate::runtime::{Executor, Features, TaskQueue, enter_context};
use crate::task::Task;

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub struct Runtime {
    queue: Arc<TaskQueue>,
    executor: Executor,
    reactor: ReactorHandle,
    io_enabled: bool,
    fs_enabled: bool,
}

impl Runtime {
    pub(crate) fn with_features(io_enabled: bool, fs_enabled: bool) -> Self {
        let queue = Arc::new(TaskQueue::new());
        let executor = Executor::new(queue.clone());
        let reactor = Arc::new(Mutex::new(Reactor::new()));

        Self {
            queue,
            executor,
            reactor,
            io_enabled,
            fs_enabled,
        }
    }

    pub fn spawn<F: Future<Output = ()> + 'static + Send>(&self, future: F) {
        let task = Task::new(future, self.queue.clone());
        self.queue.push(task);
    }

    pub fn block_on<F: Future>(&mut self, future: F) -> F::Output {
        let features = Features {
            io_enabled: self.io_enabled,
            fs_enabled: self.fs_enabled,
        };

        enter_context(self.queue.clone(), self.reactor.clone(), features, || {
            let mut future = Box::pin(future);

            let mut is_notified = false;

            fn clone_waker(data_ptr: *const ()) -> std::task::RawWaker {
                std::task::RawWaker::new(data_ptr, &VTABLE)
            }

            fn wake(data_ptr: *const ()) {
                unsafe {
                    *(data_ptr as *mut bool) = true;
                }
            }

            fn wake_by_ref(data_ptr: *const ()) {
                unsafe {
                    *(data_ptr as *mut bool) = true;
                }
            }

            fn drop_waker(_: *const ()) {}

            static VTABLE: std::task::RawWakerVTable =
                std::task::RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker);

            let raw_waker =
                std::task::RawWaker::new(&mut is_notified as *mut bool as *const (), &VTABLE);
            let waker = unsafe { std::task::Waker::from_raw(raw_waker) };
            let mut context = Context::from_waker(&waker);

            loop {
                if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                    for _ in 0..10 {
                        self.executor.run();

                        if self.queue.is_empty() {
                            break;
                        }

                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }

                    return value;
                }

                self.executor.run();

                self.reactor.lock().unwrap().poll_events();
                self.reactor.lock().unwrap().wake_ready();

                if is_notified {
                    is_notified = false;
                    continue;
                }

                if !self.queue.is_empty() {
                    continue;
                }

                self.reactor.lock().unwrap().wait_for_event();
                self.reactor.lock().unwrap().handle_events();
                self.reactor.lock().unwrap().wake_ready();
            }
        })
    }

    pub fn reactor_handle(&self) -> ReactorHandle {
        self.reactor.clone()
    }

    pub fn io_enabled(&self) -> bool {
        self.io_enabled
    }

    pub fn fs_enabled(&self) -> bool {
        self.fs_enabled
    }
}

impl Default for Runtime {
    fn default() -> Self {
        crate::builder::RuntimeBuilder::new().build()
    }
}
