pub enum ConnexionState {
    Reading,
    Writing,
}

pub(crate) struct Connexion {
    pub(crate) state: ConnexionState,
    pub(crate) out: Vec<u8>,
}

impl Connexion {
    pub(crate) fn new() -> Self {
        Self {
            state: ConnexionState::Reading,
            out: Vec::new(),
        }
    }
}
