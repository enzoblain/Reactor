use reactor::RuntimeBuilder;
use std::sync::{Arc, Mutex};

#[test]
fn test_builder_creation() {
    let rt = RuntimeBuilder::new().build();
    drop(rt);
}

#[test]
fn test_builder_simple_future() {
    let mut rt = RuntimeBuilder::new().build();
    let completed = Arc::new(Mutex::new(false));
    let completed_clone = completed.clone();

    let future = async move {
        *completed_clone.lock().unwrap() = true;
    };

    rt.block_on(future);
    assert!(*completed.lock().unwrap(), "Future should have completed");
}

#[test]
fn test_builder_immediate_result() {
    let mut rt = RuntimeBuilder::new().build();
    let value = 42;

    let future = async { value };
    let result = rt.block_on(future);

    assert_eq!(result, 42, "Future should return correct value");
}

#[test]
fn test_builder_multiple_instances() {
    let mut rt1 = RuntimeBuilder::new().build();
    let mut rt2 = RuntimeBuilder::new().build();

    let result1 = rt1.block_on(async { 10 });
    let result2 = rt2.block_on(async { 20 });

    assert_eq!(result1, 10);
    assert_eq!(result2, 20);
}

#[test]
fn test_builder_with_async_function() {
    let mut rt = RuntimeBuilder::new().build();
    let counter = Arc::new(Mutex::new(0));

    async fn increment_counter(counter: Arc<Mutex<i32>>) -> i32 {
        let mut val = counter.lock().unwrap();
        *val += 1;
        *val
    }

    let counter_clone = counter.clone();
    let result = rt.block_on(increment_counter(counter_clone));

    assert_eq!(result, 1, "Counter should be incremented");
    assert_eq!(*counter.lock().unwrap(), 1, "Shared counter should be 1");
}

#[test]
fn test_builder_with_complex_async_function() {
    let mut rt = RuntimeBuilder::new().build();
    let result = rt.block_on(complex_computation());

    assert_eq!(result, 15, "Complex computation should return 15");
}

async fn complex_computation() -> i32 {
    let mut sum = 0;

    for i in 1..=5 {
        sum += i;
    }

    sum
}

#[test]
fn test_spawn_simple_task() {
    let mut rt = RuntimeBuilder::new().build();
    let completed = Arc::new(Mutex::new(false));
    let completed_clone = completed.clone();

    rt.spawn(async move {
        *completed_clone.lock().unwrap() = true;
    });

    rt.block_on(async {});

    assert!(
        *completed.lock().unwrap(),
        "Spawned task should have completed"
    );
}

#[test]
fn test_spawn_multiple_tasks() {
    let mut rt = RuntimeBuilder::new().build();
    let counter = Arc::new(Mutex::new(0));

    for _ in 0..5 {
        let counter_clone = counter.clone();
        rt.spawn(async move {
            let mut val = counter_clone.lock().unwrap();
            *val += 1;
        });
    }

    rt.block_on(async {});

    assert_eq!(*counter.lock().unwrap(), 5, "All 5 tasks should have run");
}

#[test]
fn test_spawn_with_shared_state() {
    let mut rt = RuntimeBuilder::new().build();
    let state = Arc::new(Mutex::new(Vec::new()));

    for i in 0..3 {
        let state_clone = state.clone();
        rt.spawn(async move {
            state_clone.lock().unwrap().push(i);
        });
    }

    rt.block_on(async {});

    let final_state = state.lock().unwrap();
    assert_eq!(final_state.len(), 3, "Should have 3 values");
}

#[test]
fn test_block_on_waits_for_spawned_tasks() {
    let mut rt = RuntimeBuilder::new().build();
    let executed = Arc::new(Mutex::new(false));
    let executed_clone = executed.clone();

    rt.spawn(async move {
        *executed_clone.lock().unwrap() = true;
    });

    rt.block_on(async {});

    assert!(
        *executed.lock().unwrap(),
        "Spawned task should execute before block_on returns"
    );
}
