use super::common::Interest;
use crate::reactor::event::Event;

use libc::{
    EV_ADD, EV_CLEAR, EV_DELETE, EV_ENABLE, EVFILT_READ, EVFILT_USER, EVFILT_WRITE, NOTE_TRIGGER,
    c_long, kevent, kqueue, time_t, timespec,
};
use std::os::unix::io::RawFd;
use std::time::Duration;
use std::{io, ptr};

pub(crate) struct KqueuePoller {
    kqueue: RawFd,
    events: Vec<kevent>,
}

const WAKE_IDENT: usize = 1;

impl KqueuePoller {
    pub fn new() -> Self {
        let kqueue = unsafe { kqueue() };
        assert!(kqueue >= 0, "kqueue() failed");

        let event = kevent {
            ident: WAKE_IDENT,
            filter: EVFILT_USER,
            flags: EV_ADD | EV_ENABLE | EV_CLEAR,
            fflags: 0,
            data: 0,
            udata: ptr::null_mut(),
        };

        let ret = unsafe { kevent(kqueue, &event, 1, ptr::null_mut(), 0, ptr::null()) };
        assert!(ret == 0, "Failed to register EVFILT_USER");

        let events = Vec::with_capacity(64);

        KqueuePoller { kqueue, events }
    }

    pub fn register(&self, fd: RawFd, token: usize, interest: Interest) {
        let mut events = Vec::new();

        if interest.read {
            events.push(kevent {
                ident: fd as usize,
                filter: EVFILT_READ,
                flags: EV_ADD | EV_ENABLE,
                fflags: 0,
                data: 0,
                udata: token as *mut _,
            });
        }

        if interest.write {
            events.push(kevent {
                ident: fd as usize,
                filter: EVFILT_WRITE,
                flags: EV_ADD | EV_ENABLE,
                fflags: 0,
                data: 0,
                udata: token as *mut _,
            });
        }

        unsafe {
            kevent(
                self.kqueue,
                events.as_ptr(),
                events.len() as i32,
                ptr::null_mut(),
                0,
                ptr::null(),
            );
        }
    }

    pub fn deregister(&self, fd: RawFd) {
        let events = [
            kevent {
                ident: fd as usize,
                filter: EVFILT_READ,
                flags: EV_DELETE,
                fflags: 0,
                data: 0,
                udata: ptr::null_mut(),
            },
            kevent {
                ident: fd as usize,
                filter: EVFILT_WRITE,
                flags: EV_DELETE,
                fflags: 0,
                data: 0,
                udata: ptr::null_mut(),
            },
        ];

        unsafe {
            kevent(
                self.kqueue,
                events.as_ptr(),
                events.len() as i32,
                ptr::null_mut(),
                0,
                ptr::null(),
            );
        }
    }

    pub fn poll(&mut self, events: &mut Vec<Event>, timeout: Option<Duration>) -> io::Result<()> {
        let ts;
        let timespec_ptr = match timeout {
            Some(t) => {
                ts = timespec {
                    tv_sec: t.as_secs() as time_t,
                    tv_nsec: t.subsec_nanos() as c_long,
                };
                &ts as *const timespec
            }
            None => ptr::null(),
        };

        unsafe {
            self.events.set_len(self.events.capacity());
        }

        let n = unsafe {
            kevent(
                self.kqueue,
                ptr::null(),
                0,
                self.events.as_mut_ptr(),
                self.events.capacity() as i32,
                timespec_ptr,
            )
        };

        if n < 0 {
            let error = io::Error::last_os_error();

            if error.kind() == io::ErrorKind::Interrupted {
                return Ok(());
            }

            return Err(error);
        }

        let n = n as usize;

        unsafe {
            self.events.set_len(n);
        }

        events.clear();

        for event in &self.events[..n] {
            if event.filter == EVFILT_USER {
                continue;
            }

            let token = event.udata as usize;
            let entry = events.iter_mut().find(|e| e.token == token);

            match entry {
                Some(e) => {
                    if event.filter == EVFILT_READ {
                        e.readable = true;
                    }

                    if event.filter == EVFILT_WRITE {
                        e.writable = true;
                    }
                }
                None => {
                    events.push(Event {
                        token,
                        readable: event.filter == EVFILT_READ,
                        writable: event.filter == EVFILT_WRITE,
                    });
                }
            }
        }

        Ok(())
    }

    pub fn wake(&self) {
        let event = kevent {
            ident: WAKE_IDENT,
            filter: EVFILT_USER,
            flags: 0,
            fflags: NOTE_TRIGGER,
            data: 0,
            udata: ptr::null_mut(),
        };

        unsafe {
            kevent(self.kqueue, &event, 1, ptr::null_mut(), 0, ptr::null());
        }
    }
}
