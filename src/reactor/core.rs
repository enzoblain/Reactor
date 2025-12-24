use crate::reactor::event::Event;
use crate::reactor::io::{Connexion, ConnexionState};
use crate::reactor::socket::accept_client;

use libc::{
    EAGAIN, EVFILT_READ, EVFILT_TIMER, EVFILT_WRITE, EWOULDBLOCK, close, kqueue, read, write,
};
use std::collections::HashMap;

pub(crate) enum Entry {
    #[allow(unused)]
    Listener,
    Client(Connexion),
    // Timer,
}

pub(crate) struct Reactor {
    queue: i32,
    events: [Event; 64],
    n_events: i32,
    registry: HashMap<i32, Entry>,
}

const OUT_MAX_BYTES: usize = 8 * 1024 * 1024;

impl Reactor {
    pub(crate) fn new() -> Self {
        Self {
            queue: unsafe { kqueue() },
            events: [Event::EMPTY; 64],
            n_events: 0,
            registry: HashMap::new(),
        }
    }

    // fn register_event(&self, file_descriptor: usize, filter: i16) {
    //     let event = Event::new(file_descriptor, filter, None);
    //     event.register(self.queue);
    // }

    // fn register_read(&self, file_descriptor: i32) {
    //     self.register_event(file_descriptor as usize, EVFILT_READ);
    // }

    // fn register_write(&self, file_descriptor: i32) {
    //     self.register_event(file_descriptor as usize, EVFILT_WRITE);
    // }

    fn unregister_write(&self, file_descriptor: i32) {
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_WRITE);
    }

    pub(crate) fn wait_for_event(&mut self) {
        let n_events = Event::wait(self.queue, &mut self.events);

        self.n_events = n_events;
    }

    pub(crate) fn handle_events(&mut self) {
        for event in self.events.iter().take(self.n_events as usize) {
            let file_descriptor = event.get_ident() as i32;
            let filter = event.get_filter();

            match filter {
                EVFILT_READ
                    if matches!(self.registry.get(&(file_descriptor)), Some(Entry::Listener)) =>
                {
                    accept_client(self.queue, &mut self.registry, file_descriptor);
                }
                EVFILT_READ => {
                    let mut entry = match self.registry.remove(&file_descriptor) {
                        Some(entry) => entry,
                        None => continue,
                    };

                    let close = match &mut entry {
                        Entry::Client(conn) if matches!(conn.state, ConnexionState::Reading) => {
                            self.handle_read(file_descriptor, conn)
                        }
                        Entry::Client(_) => false,
                        _ => true,
                    };

                    if close {
                        self.cleanup(file_descriptor);
                    } else {
                        self.registry.insert(file_descriptor, entry);
                    }
                }
                EVFILT_WRITE => {
                    let mut entry = match self.registry.remove(&file_descriptor) {
                        Some(entry) => entry,
                        None => continue,
                    };

                    let close = match &mut entry {
                        Entry::Client(conn) if matches!(conn.state, ConnexionState::Writing) => {
                            self.handle_write(file_descriptor, conn)
                        }
                        Entry::Client(_) => false,
                        _ => true,
                    };

                    if close {
                        self.cleanup(file_descriptor);
                    } else {
                        self.registry.insert(file_descriptor, entry);
                    }
                }
                EVFILT_TIMER => {
                    todo!()
                }
                _ => {}
            }
        }
    }

    fn handle_read(&self, file_descriptor: i32, connexion: &mut Connexion) -> bool {
        let mut buf = [0u8; 1024];
        let res = unsafe { read(file_descriptor, buf.as_mut_ptr() as *mut _, buf.len()) };

        if res == 0 {
            return true;
        }

        if res < 0 {
            let err = errno();
            if err == EAGAIN || err == EWOULDBLOCK {
                return false;
            }

            return true;
        }

        let add_len = res as usize;
        if connexion.out.len().saturating_add(add_len) > OUT_MAX_BYTES {
            return true;
        }

        connexion.out.extend_from_slice(&buf[..add_len]);
        connexion.state = ConnexionState::Writing;

        false
    }

    fn handle_write(&self, file_descriptor: i32, connexion: &mut Connexion) -> bool {
        let res = unsafe {
            write(
                file_descriptor,
                connexion.out.as_mut_ptr() as *mut _,
                connexion.out.len(),
            )
        };

        if res < 0 {
            let err = errno();
            if err == EAGAIN || err == EWOULDBLOCK {
                return false;
            }

            return true;
        }

        connexion.out.drain(..res as usize);

        if connexion.out.is_empty() {
            self.unregister_write(file_descriptor);
            connexion.state = ConnexionState::Reading;
        }

        false
    }

    fn cleanup(&self, file_descriptor: i32) {
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_READ);
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_WRITE);
        unsafe { close(file_descriptor) };
    }
}

fn errno() -> i32 {
    unsafe { *libc::__error() }
}
