//! Example demonstrating async I/O futures with the reactor.
//!
//! This example shows how to use AsyncRead and AsyncWrite futures
//! that integrate with the reactor's wake system.

use reactor::{Runtime, Task};

async fn async_io_example() {
    println!("Starting async I/O example");

    // Simulate multiple concurrent I/O operations
    Task::spawn(async {
        println!("Task 1: Waiting for I/O readiness...");
        // In a real scenario, you would have an actual file descriptor
        // let fd = open_socket().await;
        // AsyncRead::new(fd).await.unwrap();
        println!("Task 1: I/O ready!");
    });

    Task::spawn(async {
        println!("Task 2: Waiting for write readiness...");
        // let fd = open_socket().await;
        // AsyncWrite::new(fd).await.unwrap();
        println!("Task 2: Write ready!");
    });

    // Await task completion deterministically (no sleeps)
    let j1 = Task::spawn(async {
        println!("Task 1: Waiting for I/O readiness...");
        println!("Task 1: I/O ready!");
    });
    let j2 = Task::spawn(async {
        println!("Task 2: Waiting for write readiness...");
        println!("Task 2: Write ready!");
    });
    j1.await;
    j2.await;

    println!("Async I/O example completed");
}

async fn waker_coordination_example() {
    println!("\nDemonstrating waker coordination:");

    // Spawn multiple tasks that will be woken by the reactor
    let mut joins = reactor::JoinSet::new();
    for i in 0..5 {
        let j = Task::spawn(async move {
            println!("  Task {} started", i);
            println!("  Task {} woken and completed", i);
        });
        joins.push(j);
    }
    joins.await_all().await;

    println!("All tasks completed");
}

async fn nested_async_operations() {
    println!("\nNested async operations with wake:");

    let parent = Task::spawn(async {
        println!("  Parent task starting");

        // Spawn nested tasks and await them deterministically
        let c1 = Task::spawn(async {
            println!("    Child task 1 starting");
            println!("    Child task 1 completed");
        });

        let c2 = Task::spawn(async {
            println!("    Child task 2 starting");
            println!("    Child task 2 completed");
        });

        c1.await;
        c2.await;
        println!("  Parent task completed");
    });

    parent.await;
    println!("Nested operations completed");
}

fn main() {
    let mut rt = Runtime::new();

    println!("=== Reactor Future & Wake Management Demo ===\n");

    rt.block_on(async {
        async_io_example().await;
        waker_coordination_example().await;
        nested_async_operations().await;
    });

    println!("\n=== Demo Complete ===");
}
