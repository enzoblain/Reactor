use reactor::time::sleep;
use reactor::time::timeout;
use reactor::{RuntimeBuilder, Task};
use std::time::Duration;

#[test]
fn test_timeout_completes_before_deadline() {
    let mut rt = RuntimeBuilder::new().enable_io().build();

    let result = rt.block_on(async {
        let handle = Task::spawn(async {
            sleep(Duration::from_millis(10)).await;
            123
        });
        timeout(Duration::from_millis(50), handle).await
    });

    assert!(
        matches!(result, Ok(v) if v == 123),
        "Timeout should return Ok(123)"
    );
}

#[test]
fn test_timeout_expires() {
    let mut rt = RuntimeBuilder::new().enable_io().build();

    let result = rt.block_on(async {
        let handle = Task::spawn(async {
            sleep(Duration::from_millis(100)).await;
            456
        });
        timeout(Duration::from_millis(20), handle).await
    });

    assert!(
        result.is_err(),
        "Timeout should return an error when deadline is exceeded"
    );
}
