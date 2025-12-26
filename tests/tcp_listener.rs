use reactor::net::tcp_listener::TcpListener;
use reactor::{Runtime, Task};
use std::io::{Read, Write};
use std::net::TcpStream as StdTcpStream;
use std::sync::{Arc, Mutex};

#[test]
fn tcp_accept_and_echo() {
    let mut rt = Runtime::new();

    let result = Arc::new(Mutex::new(Vec::new()));
    let result_clone = result.clone();

    rt.block_on(async move {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let addr_str = format!("{}:{}", addr.ip(), addr.port());

        let handle = Task::spawn(async move {
            let (stream, _peer) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4];
            let n = stream.read(&mut buf).await.expect("read");
            assert_eq!(&buf[..n], b"ping");
            stream.write_all(b"pong").await.expect("write_all");
        });

        std::thread::spawn(move || {
            let mut c = StdTcpStream::connect(&addr_str).expect("connect");
            c.write_all(b"ping").expect("write");
            let mut buf = [0u8; 4];
            c.read_exact(&mut buf).expect("read_exact");
            result_clone.lock().unwrap().extend_from_slice(&buf);
        })
        .join()
        .unwrap();

        // Wait for the spawned task to complete
        handle.await;
    });

    assert_eq!(&*result.lock().unwrap(), b"pong");
}

#[test]
fn tcp_write_all_large_payload() {
    let mut rt = Runtime::new();

    let payload = vec![7u8; 16 * 1024];
    let payload_len = payload.len();
    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    rt.block_on(async move {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let addr_str = format!("{}:{}", addr.ip(), addr.port());

        let handle = Task::spawn(async move {
            let (stream, _peer) = listener.accept().await.expect("accept");
            stream.write_all(&payload).await.expect("write_all");
        });

        std::thread::spawn(move || {
            let mut c = StdTcpStream::connect(&addr_str).expect("connect");
            let mut buf = vec![0u8; payload_len];
            c.read_exact(&mut buf).expect("read_exact");
            *received_clone.lock().unwrap() = buf;
        })
        .join()
        .unwrap();

        // Wait for the spawned task to complete
        handle.await;
    });

    assert_eq!(received.lock().unwrap().len(), payload_len);
    assert!(received.lock().unwrap().iter().all(|&b| b == 7));
}
