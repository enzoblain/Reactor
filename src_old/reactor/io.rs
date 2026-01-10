pub(crate) enum ConnectionState {
    Reading,
    Writing,
}

pub(crate) struct Connection {
    pub(crate) state: ConnectionState,
    pub(crate) out: Vec<u8>,
}

impl Connection {
    pub(crate) fn new() -> Self {
        Self {
            state: ConnectionState::Reading,
            out: Vec::new(),
        }
    }
}
