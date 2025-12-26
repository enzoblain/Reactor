use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use reactor::{AsyncRead, AsyncWrite, Runtime, Task};

#[test]
fn test_async_read_pipe_wakes() {
    let mut rt = Runtime::new();
    let ok = Arc::new(AtomicBool::new(false));
    let ok2 = ok.clone();

    rt.block_on(async move {
        // Create a pipe (read end, write end)
        let mut fds = [0i32; 2];
        let res = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(res, 0, "pipe() failed");
        let rfd = fds[0];
        let wfd = fds[1];

        let handle = Task::spawn(async move {
            AsyncRead::new(rfd).await.unwrap();
            ok2.store(true, Ordering::SeqCst);
        });

        // Write a byte to trigger read readiness
        let buf = [1u8; 1];
        let wrote = unsafe { libc::write(wfd, buf.as_ptr() as *const _, 1) };
        assert_eq!(wrote, 1);

        unsafe {
            libc::close(wfd);
        }

        // Wait for the task to complete
        handle.await;
    });

    assert!(ok.load(Ordering::SeqCst));
}

#[test]
fn test_async_write_pipe_wakes() {
    let mut rt = Runtime::new();
    let ok = Arc::new(AtomicBool::new(false));
    let ok2 = ok.clone();

    rt.block_on(async move {
        let mut fds = [0i32; 2];
        let res = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(res, 0, "pipe() failed");
        let _rfd = fds[0];
        let wfd = fds[1];

        let handle = Task::spawn(async move {
            AsyncWrite::new(wfd).await.unwrap();
            ok2.store(true, Ordering::SeqCst);
        });

        // Wait for the task to complete
        handle.await;
    });

    assert!(ok.load(Ordering::SeqCst));
}
