use crate::reactor::event::Event;
use crate::reactor::io::{Connexion, ConnexionState};
use crate::reactor::socket::accept_client;

use libc::{
    EAGAIN, EVFILT_READ, EVFILT_TIMER, EVFILT_WRITE, EWOULDBLOCK, close, kqueue, read, write,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr;
use std::task::Waker;

thread_local! {
    /// Thread-local pointer to the current Runtime's reactor
    pub(crate) static CURRENT_REACTOR_PTR: RefCell<*mut Reactor> = const { RefCell::new(ptr::null_mut()) };
}

pub(crate) fn set_current_reactor(r: &mut Reactor) {
    CURRENT_REACTOR_PTR.with(|cell| {
        *cell.borrow_mut() = r as *mut Reactor;
    });
}

pub(crate) fn with_current_reactor<R>(f: impl FnOnce(&mut Reactor) -> R) -> Option<R> {
    CURRENT_REACTOR_PTR.with(|cell| {
        let ptr = *cell.borrow();
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { f(&mut *ptr) })
        }
    })
}

pub(crate) enum Entry {
    #[allow(unused)]
    Listener,
    Client(Connexion),
    Waiting(Waker),
    // Timer,
}

pub(crate) struct Reactor {
    queue: i32,
    events: [Event; 64],
    n_events: i32,
    registry: HashMap<i32, Entry>,
    wakers: Vec<Waker>,
}

const OUT_MAX_BYTES: usize = 8 * 1024 * 1024;

impl Reactor {
    pub(crate) fn new() -> Self {
        Self {
            queue: unsafe { kqueue() },
            events: [Event::EMPTY; 64],
            n_events: 0,
            registry: HashMap::new(),
            wakers: Vec::new(),
        }
    }

    pub(crate) fn register_read(&mut self, file_descriptor: i32, waker: Waker) {
        let event = Event::new(file_descriptor as usize, EVFILT_READ, None);
        event.register(self.queue);

        self.registry.insert(file_descriptor, Entry::Waiting(waker));
    }

    pub(crate) fn register_write(&mut self, file_descriptor: i32, waker: Waker) {
        let event = Event::new(file_descriptor as usize, EVFILT_WRITE, None);
        event.register(self.queue);

        self.registry.insert(file_descriptor, Entry::Waiting(waker));
    }

    fn unregister_write(&self, file_descriptor: i32) {
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_WRITE);
    }

    pub(crate) fn wait_for_event(&mut self) {
        let n_events = Event::wait(self.queue, &mut self.events);

        self.n_events = n_events;
    }

    /// Polls for I/O events without blocking and handles them if present.
    pub(crate) fn poll_events(&mut self) {
        let n_events = Event::try_wait(self.queue, &mut self.events);
        if n_events <= 0 {
            return;
        }
        self.n_events = n_events;
        self.handle_events();
    }

    pub(crate) fn wake_ready(&mut self) {
        for waker in self.wakers.drain(..) {
            waker.wake();
        }
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

                    match &mut entry {
                        Entry::Waiting(waker) => {
                            self.wakers.push(waker.clone());
                            continue;
                        }
                        Entry::Client(conn) if matches!(conn.state, ConnexionState::Reading) => {
                            let close = self.handle_read(file_descriptor, conn);
                            if close {
                                self.cleanup(file_descriptor);
                            } else {
                                self.registry.insert(file_descriptor, entry);
                            }
                        }
                        Entry::Client(_) => {
                            self.registry.insert(file_descriptor, entry);
                        }
                        _ => {
                            self.cleanup(file_descriptor);
                        }
                    }
                }
                EVFILT_WRITE => {
                    let mut entry = match self.registry.remove(&file_descriptor) {
                        Some(entry) => entry,
                        None => continue,
                    };

                    match &mut entry {
                        Entry::Waiting(waker) => {
                            self.wakers.push(waker.clone());
                            continue;
                        }
                        Entry::Client(conn) if matches!(conn.state, ConnexionState::Writing) => {
                            let close = self.handle_write(file_descriptor, conn);
                            if close {
                                self.cleanup(file_descriptor);
                            } else {
                                self.registry.insert(file_descriptor, entry);
                            }
                        }
                        Entry::Client(_) => {
                            self.registry.insert(file_descriptor, entry);
                        }
                        _ => {
                            self.cleanup(file_descriptor);
                        }
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
