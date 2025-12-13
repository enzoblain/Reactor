use crate::Runtime;

pub enum BuildError {}

pub struct Builder {
    worker_threads: usize,
}

impl Builder {
    pub fn worker_threads(mut self, n: usize) -> Self {
        self.worker_threads = n;
        self
    }

    pub fn build(self) -> Result<Runtime, BuildError> {
        Ok(Runtime::default())
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder { worker_threads: 1 }
    }
}
