use crate::reactor::event::Event;
use crate::reactor::io::{Connection, ConnectionState};
use crate::reactor::socket::accept_client;
use crate::utils::Slab;

use std::process::Command;
use std::sync::mpsc::Receiver;

pub(crate) struct Reactor {
    receiver: Receiver<Command>,

    poller: Poller,
    events: Vec<Event>,

    timers: Slab<TimerEntry>,
    io: Slab<IoEntry>,
}
