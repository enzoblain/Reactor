use crate::reactor::core::ReactorHandle;
use crate::runtime::workstealing::{CURRENT_INJECTOR, Injector};

use std::cell::RefCell;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Features {
    pub(crate) io_enabled: bool,
    pub(crate) fs_enabled: bool,
}

thread_local! {
    pub(crate) static CURRENT_REACTOR: RefCell<Option<ReactorHandle>> = const { RefCell::new(None) };
    pub(crate) static CURRENT_FEATURES: RefCell<Option<Features>> = const { RefCell::new(None) };
}

pub(crate) fn enter_context<F, R>(
    injector: Arc<Injector>,
    reactor: ReactorHandle,
    features: Features,
    function: F,
) -> R
where
    F: FnOnce() -> R,
{
    CURRENT_INJECTOR.with(|current_injector| {
        CURRENT_REACTOR.with(|current_reactor| {
            CURRENT_FEATURES.with(|current_features| {
                let prev_injector = current_injector.borrow_mut().replace(injector);
                let prev_reactor = current_reactor.borrow_mut().replace(reactor);
                let prev_features = current_features.borrow_mut().replace(features);

                let result = function();

                *current_injector.borrow_mut() = prev_injector;
                *current_reactor.borrow_mut() = prev_reactor;
                *current_features.borrow_mut() = prev_features;

                result
            })
        })
    })
}

pub(crate) fn current_reactor_io() -> ReactorHandle {
    ensure_feature(|f| f.io_enabled, "I/O", "RuntimeBuilder::enable_io()");

    current_reactor_inner()
}

pub(crate) fn current_reactor_fs() -> ReactorHandle {
    ensure_feature(
        |f| f.fs_enabled,
        "filesystem",
        "RuntimeBuilder::enable_fs()",
    );

    current_reactor_inner()
}

fn ensure_feature(check: impl Fn(&Features) -> bool, name: &str, hint: &str) {
    CURRENT_FEATURES.with(|features| {
        let enabled = features.borrow().as_ref().map(check).unwrap_or(false);

        if !enabled {
            panic!("{} support not enabled. Use {}.", name, hint);
        }
    })
}

fn current_reactor_inner() -> ReactorHandle {
    CURRENT_REACTOR.with(|current| {
        current.borrow().clone().expect(
            "No reactor in current context. I/O operations must be called within Runtime::block_on",
        )
    })
}
