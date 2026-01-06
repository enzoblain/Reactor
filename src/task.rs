use crate::runtime::{CURRENT_QUEUE, TaskQueue, make_waker};

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub struct Task<T: Send> {
    future: Mutex<Option<Pin<Box<dyn Future<Output = T> + Send>>>>,
    pub(crate) result: Mutex<Option<T>>,
    pub(crate) queue: Arc<TaskQueue>,
    pub(crate) completed: AtomicBool,
    pub(crate) waiters: Mutex<Vec<Waker>>,
}

impl<T: 'static + Send> Task<T> {
    pub(crate) fn new<F>(fut: F, queue: Arc<TaskQueue>) -> Arc<Self>
    where
        F: Future<Output = T> + 'static + Send,
    {
        Arc::new(Task {
            future: Mutex::new(Some(Box::pin(fut))),
            result: Mutex::new(None),
            queue,
            completed: AtomicBool::new(false),
            waiters: Mutex::new(Vec::new()),
        })
    }

    pub fn poll(self: &Arc<Self>) {
        let waker = make_waker(self.clone());
        let mut context = Context::from_waker(&waker);

        let mut future_slot = self.future.lock().unwrap();

        if let Some(mut future) = future_slot.take() {
            match future.as_mut().poll(&mut context) {
                Poll::Pending => {
                    *future_slot = Some(future);
                }
                Poll::Ready(val) => {
                    *self.result.lock().unwrap() = Some(val);
                    self.completed.store(true, Ordering::Release);

                    let mut waiters = self.waiters.lock().unwrap();
                    for w in waiters.drain(..) {
                        w.wake();
                    }
                }
            }
        }
    }

    pub fn spawn<F>(future: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + 'static + Send,
    {
        CURRENT_QUEUE.with(|current| {
            let queue = current
                .borrow()
                .as_ref()
                .expect("Task::spawn() called outside of a runtime context")
                .clone();

            let task: Arc<Task<T>> = Task::new(future, queue.clone());
            let runnable: Arc<dyn Runnable> = task.clone();

            queue.push(runnable);

            JoinHandle { task }
        })
    }
}

pub(crate) trait Runnable: Send + Sync {
    fn poll(self: Arc<Self>);
}

impl<T: 'static + Send> Runnable for Task<T> {
    fn poll(self: Arc<Self>) {
        Task::poll(&self);
    }
}

pub struct JoinHandle<T: Send> {
    task: Arc<Task<T>>,
}

impl<T: Send> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.task.completed.load(Ordering::SeqCst) {
            let result = self
                .task
                .result
                .lock()
                .unwrap()
                .take()
                .expect("task completed but result missing");

            return Poll::Ready(result);
        }

        let mut ws = self.task.waiters.lock().unwrap();
        ws.push(cx.waker().clone());

        Poll::Pending
    }
}

pub struct JoinSet<T: Send> {
    handles: Vec<JoinHandle<T>>,
}

impl<T: Send> JoinSet<T> {
    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    pub fn push(&mut self, handle: JoinHandle<T>) {
        self.handles.push(handle);
    }

    pub async fn await_all(&mut self) {
        for handle in self.handles.drain(..) {
            handle.await;
        }
    }
}

impl<T: Send> Default for JoinSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
