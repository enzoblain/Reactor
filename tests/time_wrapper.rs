use reactor::time::sleep;
use reactor::time::wrapper::Time;
use reactor::{RuntimeBuilder, Task};
use std::time::Duration;

#[test]
fn test_time_wrapper_with_sleep() {
    let mut rt = RuntimeBuilder::new().enable_io().build();

    let (_, elapsed) = rt.block_on(async {
        let handle = Task::spawn(async {
            sleep(Duration::from_millis(50)).await;
        });

        Time::new(handle).await
    });

    assert!(
        elapsed >= Duration::from_millis(50),
        "Time wrapper should measure at least the sleep duration"
    );
}
