use super::common::Interest;

use libc::{
    EV_ADD, EV_CLEAR, EV_DELETE, EV_ENABLE, EVFILT_READ, EVFILT_USER, EVFILT_WRITE, kevent, kqueue,
};
use std::os::unix::io::RawFd;
use std::ptr;

pub(crate) struct KqueuePoller {
    kqueue: RawFd,
    events: Vec<kevent>,
}

const WAKE_IDENT: usize = 1;

impl KqueuePoller {
    pub fn new() -> Self {
        let kqueue = unsafe { kqueue() };
        assert!(kqueue >= 0, "kqueue() failed");

        let event = libc::kevent {
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
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            );
        }
    }

    pub fn deregister(&self, fd: RawFd) {
        let mut events = Vec::new();

        events.push(kevent {
            ident: fd as usize,
            filter: EVFILT_READ,
            flags: EV_DELETE,
            fflags: 0,
            data: 0,
            udata: ptr::null_mut(),
        });

        unsafe {
            kevent(
                self.kqueue,
                events.as_ptr(),
                events.len() as i32,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            );
        }
    }
    // pub fn poll(&mut self, events: &mut Vec<Event>, timeout: Option<Duration>);
    // pub fn wake(&self);
}
