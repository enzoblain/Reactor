use crate::reactor::core::ReactorHandle;
use crate::runtime::context::Features;
use crate::runtime::workstealing::{Injector, LocalQueue, Worker};

use std::sync::Arc;
use std::thread::JoinHandle;

pub(crate) struct Executor {
    workers: Vec<Arc<Worker>>,
    handles: Vec<JoinHandle<()>>,
}

impl Executor {
    pub(crate) fn new(queue: Arc<Injector>, reactor: ReactorHandle, features: Features) -> Self {
        let num_workers = num_cpus().max(1);
        let locals = Arc::new(
            (0..num_workers)
                .map(|_| LocalQueue::new())
                .collect::<Vec<_>>(),
        );

        let workers = (0..num_workers)
            .map(|id| {
                Arc::new(Worker {
                    id,
                    locals: locals.clone(),
                    injector: queue.clone(),
                    reactor: reactor.clone(),
                    features,
                })
            })
            .collect();

        Self {
            workers,
            handles: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        for worker in &self.workers {
            let worker = worker.clone();
            let handle = std::thread::spawn(move || worker.run());
            self.handles.push(handle);
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}
