use reactor::{Runtime, sleep};
use std::time::{Duration, Instant};

#[test]
fn test_sleep_basic() {
    let mut rt = Runtime::new();

    let start = Instant::now();
    rt.block_on(async {
        sleep(Duration::from_millis(50)).await;
    });
    let elapsed = start.elapsed();

    assert!(
        elapsed >= Duration::from_millis(50),
        "Sleep should wait at least the specified duration"
    );
}

#[test]
fn test_sleep_zero_duration() {
    let mut rt = Runtime::new();

    let start = Instant::now();
    rt.block_on(async {
        sleep(Duration::from_millis(0)).await;
    });
    let elapsed = start.elapsed();

    // Should complete almost immediately
    assert!(
        elapsed < Duration::from_millis(10),
        "Zero duration sleep should be fast"
    );
}

#[test]
fn test_sleep_in_function() {
    let mut rt = Runtime::new();
    let start = Instant::now();

    rt.block_on(async {
        sleep_and_record(start).await;
    });
}

async fn sleep_and_record(start: Instant) {
    let elapsed_before = start.elapsed();
    sleep(Duration::from_millis(30)).await;
    let elapsed_after = start.elapsed();

    assert!(elapsed_after - elapsed_before >= Duration::from_millis(30));
}
