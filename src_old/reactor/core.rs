use crate::reactor::event::Event;
use crate::reactor::io::{Connection, ConnectionState};
use crate::reactor::socket::accept_client;

use libc::{
    EAGAIN, EVFILT_READ, EVFILT_TIMER, EVFILT_WRITE, EWOULDBLOCK, close, kqueue, read, write,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::time::Duration;

pub(crate) enum Entry {
    #[allow(unused)]
    Listener,
    Client(Connection),
    Waiting(Waker),
}

pub struct Reactor {
    queue: i32,
    events: [Event; 64],
    n_events: i32,
    registry: HashMap<i32, Entry>,
    timers: HashMap<usize, (Waker, Arc<AtomicBool>)>,
    next_timer_id: usize,
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
            timers: HashMap::new(),
            next_timer_id: 1,
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

    pub(crate) fn register_timer(&mut self, duration: Duration, waker: Waker) {
        self.register_timer_with_callback(duration, waker, Arc::new(AtomicBool::new(false)));
    }

    pub(crate) fn register_timer_with_callback(
        &mut self,
        duration: Duration,
        waker: Waker,
        expired: Arc<AtomicBool>,
    ) {
        let milliseconds = duration.as_millis().clamp(0, isize::MAX as u128) as isize;
        let id = self.next_timer_id;
        self.next_timer_id = self.next_timer_id.wrapping_add(1).max(1);

        let event = Event::new(id, EVFILT_TIMER, Some(milliseconds));
        event.register(self.queue);

        self.timers.insert(id, (waker, expired));
    }

    fn unregister_write(&self, file_descriptor: i32) {
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_WRITE);
    }

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
                            self.registry.insert(file_descriptor, entry);
                            continue;
                        }
                        Entry::Client(connection)
                            if matches!(connection.state, ConnectionState::Reading) =>
                        {
                            let should_close = self.handle_read(file_descriptor, connection);

                            if should_close {
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
                            self.registry.insert(file_descriptor, entry);
                            continue;
                        }
                        Entry::Client(connection)
                            if matches!(connection.state, ConnectionState::Writing) =>
                        {
                            let should_close = self.handle_write(file_descriptor, connection);

                            if should_close {
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
                    let timer_id = event.get_ident();

                    if let Some((waker, expired)) = self.timers.remove(&timer_id) {
                        expired.store(true, Ordering::Release);
                        self.wakers.push(waker);
                    }
                }

                _ => {}
            }
        }
    }

    fn handle_read(&self, file_descriptor: i32, connection: &mut Connection) -> bool {
        let mut buffer = [0u8; 1024];
        let result = unsafe { read(file_descriptor, buffer.as_mut_ptr() as *mut _, buffer.len()) };

        if result == 0 {
            return true;
        }

        if result < 0 {
            let error = get_errno();

            if error == EAGAIN || error == EWOULDBLOCK {
                return false;
            }

            return true;
        }

        let bytes_read = result as usize;

        if connection.out.len().saturating_add(bytes_read) > OUT_MAX_BYTES {
            return true;
        }

        connection.out.extend_from_slice(&buffer[..bytes_read]);
        connection.state = ConnectionState::Writing;

        false
    }

    fn handle_write(&self, file_descriptor: i32, connection: &mut Connection) -> bool {
        let result = unsafe {
            write(
                file_descriptor,
                connection.out.as_mut_ptr() as *mut _,
                connection.out.len(),
            )
        };

        if result < 0 {
            let error = get_errno();

            if error == EAGAIN || error == EWOULDBLOCK {
                return false;
            }

            return true;
        }

        let bytes_written = result as usize;
        connection.out.drain(..bytes_written);

        if connection.out.is_empty() {
            self.unregister_write(file_descriptor);
            connection.state = ConnectionState::Reading;
        }

        false
    }

    fn cleanup(&self, file_descriptor: i32) {
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_READ);
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_WRITE);
        unsafe { close(file_descriptor) };
    }
}

pub(crate) fn get_errno() -> i32 {
    unsafe { *libc::__error() }
}
