use crate::runtime::make_waker;
use crate::runtime::workstealing::{CURRENT_INJECTOR, Injector};

use std::cell::UnsafeCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub struct Task<T: Send + Sync + 'static> {
    pub(crate) future: UnsafeCell<Pin<Box<dyn Future<Output = T> + Send>>>,

    pub(crate) result: UnsafeCell<Option<T>>,
    pub(crate) completed: AtomicBool,

    pub(crate) injector: Arc<Injector>,
    pub(crate) inqueue: AtomicBool,

    pub(crate) waiters: Mutex<Vec<Waker>>,
}

unsafe impl<T> Sync for Task<T> where T: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> Task<T> {
    pub(crate) fn new<F>(fut: F, injector: Arc<Injector>) -> Arc<Self>
    where
        F: Future<Output = T> + Send + 'static,
    {
        Arc::new(Task {
            future: UnsafeCell::new(Box::pin(fut)),
            result: UnsafeCell::new(None),
            injector,
            inqueue: AtomicBool::new(false),
            completed: AtomicBool::new(false),
            waiters: Mutex::new(Vec::new()),
        })
    }

    pub fn poll(self: &Arc<Self>) {
        if self.completed.load(Ordering::Acquire) {
            return;
        }

        let waker = make_waker(self.clone());
        let mut context = Context::from_waker(&waker);

        let future = unsafe { &mut *self.future.get() };

        match future.as_mut().poll(&mut context) {
            Poll::Pending => {
                self.inqueue.store(false, Ordering::Release);
            }
            Poll::Ready(val) => {
                unsafe {
                    *self.result.get() = Some(val);
                }

                self.completed.store(true, Ordering::Release);

                self.injector.task_completed();

                let mut waiters = self.waiters.lock().unwrap();
                waiters.drain(..).for_each(|w| w.wake());
            }
        }
    }

    pub fn spawn<F>(future: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
    {
        CURRENT_INJECTOR.with(|cell| {
            let injector = cell
                .borrow()
                .as_ref()
                .expect("Task::spawn() called outside of a runtime context")
                .clone();

            let task = Task::new(future, injector.clone());
            let r: Arc<dyn Runnable> = task.clone();

            task.inqueue.store(true, Ordering::Release);
            injector.push(r);

            JoinHandle { task }
        })
    }
}

pub(crate) trait Runnable: Send + Sync {
    fn poll(self: Arc<Self>);
}

impl<T: Send + Sync + 'static> Runnable for Task<T> {
    fn poll(self: Arc<Self>) {
        Task::poll(&self);
    }
}

pub struct JoinHandle<T: Send + Sync + 'static> {
    task: Arc<Task<T>>,
}

impl<T: Send + Sync> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.task.completed.load(Ordering::Acquire) {
            let result = unsafe { (*self.task.result.get()).take() }.unwrap();

            return Poll::Ready(result);
        }

        let mut waiters = self.task.waiters.lock().unwrap();
        waiters.push(cx.waker().clone());

        Poll::Pending
    }
}
