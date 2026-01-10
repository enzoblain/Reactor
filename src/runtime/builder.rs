use super::Runtime;

pub struct RuntimeBuilder {}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(self) -> Runtime {
        Runtime::new()
    }
}
