use crate::core::task::Runnable;
use crate::reactor::core::{Reactor, ReactorHandle};
use crate::runtime::workstealing::Injector;
use crate::runtime::{Executor, Features, enter_context};
use crate::{RuntimeBuilder, Task};

use std::future::Future;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub struct Runtime {
    injector: Arc<Injector>,
    reactor: ReactorHandle,
    io_enabled: bool,
    fs_enabled: bool,
}

impl Runtime {
    pub fn spawn<F: Future<Output = ()> + Send + 'static>(&self, future: F) {
        let injector = self.injector.clone();
        let task = Task::new(future, injector.clone());
        let r: Arc<dyn Runnable> = task.clone();
        task.inqueue.store(true, Ordering::Release);
        injector.push(r);
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let features = Features {
            io_enabled: self.io_enabled,
            fs_enabled: self.fs_enabled,
        };

        enter_context(
            self.injector.clone(),
            self.reactor.clone(),
            features,
            || {
                let mut future = Box::pin(future);
                let mut is_notified = false;
                let mut root_done = false;
                let mut root_value = None;

                fn clone_waker(data: *const ()) -> std::task::RawWaker {
                    std::task::RawWaker::new(data, &VTABLE)
                }
                fn wake(data: *const ()) {
                    unsafe {
                        *(data as *mut bool) = true;
                    }
                }
                fn wake_by_ref(data: *const ()) {
                    unsafe {
                        *(data as *mut bool) = true;
                    }
                }
                fn drop_waker(_: *const ()) {}

                static VTABLE: std::task::RawWakerVTable =
                    std::task::RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker);

                let raw =
                    std::task::RawWaker::new(&mut is_notified as *mut bool as *const (), &VTABLE);
                let waker = unsafe { std::task::Waker::from_raw(raw) };
                let mut cx = Context::from_waker(&waker);

                loop {
                    if !root_done && let Poll::Ready(v) = future.as_mut().poll(&mut cx) {
                        root_done = true;
                        root_value = Some(v);
                    }

                    {
                        let mut r = self.reactor.lock().unwrap();
                        r.poll_events();
                        r.wake_ready();
                    }

                    if root_done && self.injector.is_idle() {
                        return root_value.unwrap();
                    }

                    while let Some(task) = self.injector.pop() {
                        task.poll();

                        {
                            let mut r = self.reactor.lock().unwrap();
                            r.poll_events();
                            r.wake_ready();
                        }

                        if root_done && self.injector.is_idle() {
                            return root_value.unwrap();
                        }
                    }

                    if root_done && self.injector.is_idle() {
                        return root_value.unwrap();
                    }

                    if is_notified {
                        is_notified = false;
                        continue;
                    }

                    std::thread::yield_now();
                }
            },
        )
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

impl Drop for Runtime {
    fn drop(&mut self) {
        self.injector.shutdown();
    }
}

impl Default for Runtime {
    fn default() -> Self {
        RuntimeBuilder::new().build()
    }
}
