use std::sync::mpsc::Sender;

pub(crate) struct ReactorHandle {
    transmitter: Sender<Command>,
    wake: WakeHandle,
}
