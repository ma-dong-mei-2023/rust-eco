use rust_eco::{channel, sync, spawn, time, Eco};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eco = Eco::new();

    eco.run(async {
        println!("Channels and synchronization demo");

        // 1. Basic channel communication
        println!("\n--- Basic Channel Communication ---");
        let channel = channel::Channel::new();
        let sender = channel.sender();
        let receiver = channel.receiver();

        let producer = spawn({
            let sender = sender.clone();
            async move {
                for i in 1..=5 {
                    let message = format!("Message {}", i);
                    println!("Sending: {}", message);
                    sender.send(message)?;
                    time::sleep_secs(0.5).await;
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        let consumer = spawn({
            let receiver = receiver.clone();
            async move {
                while let Some(message) = receiver.recv().await {
                    println!("Received: {}", message);
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        // Wait for producer to finish
        producer.await??;
        // Give consumer time to process remaining messages
        time::sleep_secs(1.0).await;

        // 2. Bounded channel with backpressure
        println!("\n--- Bounded Channel with Backpressure ---");
        let bounded_channel = channel::Channel::bounded(2);
        let bounded_sender = bounded_channel.sender();
        let bounded_receiver = bounded_channel.receiver();

        let fast_producer = spawn({
            let sender = bounded_sender;
            async move {
                for i in 1..=6 {
                    println!("Trying to send bounded message {}", i);
                    sender.send(format!("Bounded message {}", i)).await?;
                    println!("Sent bounded message {}", i);
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        let slow_consumer = spawn({
            let receiver = bounded_receiver;
            async move {
                while let Some(message) = receiver.recv().await {
                    println!("Slowly processing: {}", message);
                    time::sleep_secs(1.0).await; // Slow processing
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        fast_producer.await??;
        time::sleep_secs(3.0).await; // Let consumer finish

        // 3. Broadcast channel
        println!("\n--- Broadcast Channel ---");
        let broadcast = channel::Broadcast::new(10);
        let mut receiver1 = broadcast.subscribe();
        let mut receiver2 = broadcast.subscribe();
        let mut receiver3 = broadcast.subscribe();

        let broadcaster = spawn({
            let broadcast = broadcast.clone();
            async move {
                for i in 1..=3 {
                    let message = format!("Broadcast {}", i);
                    println!("Broadcasting: {}", message);
                    broadcast.send(message)?;
                    time::sleep_secs(0.5).await;
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        let listener1 = spawn(async move {
            while let Ok(message) = receiver1.recv().await {
                println!("Listener 1 received: {}", message);
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        });

        let listener2 = spawn(async move {
            while let Ok(message) = receiver2.recv().await {
                println!("Listener 2 received: {}", message);
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        });

        let listener3 = spawn(async move {
            while let Ok(message) = receiver3.recv().await {
                println!("Listener 3 received: {}", message);
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        });

        broadcaster.await??;
        time::sleep_secs(1.0).await;

        // 4. WaitGroup synchronization
        println!("\n--- WaitGroup Synchronization ---");
        let wait_group = sync::WaitGroup::new();
        wait_group.add(3).await;

        let worker1 = spawn({
            let wg = wait_group.clone();
            async move {
                println!("Worker 1 starting...");
                time::sleep_secs(1.0).await;
                println!("Worker 1 finished");
                wg.done().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        let worker2 = spawn({
            let wg = wait_group.clone();
            async move {
                println!("Worker 2 starting...");
                time::sleep_secs(2.0).await;
                println!("Worker 2 finished");
                wg.done().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        let worker3 = spawn({
            let wg = wait_group.clone();
            async move {
                println!("Worker 3 starting...");
                time::sleep_secs(1.5).await;
                println!("Worker 3 finished");
                wg.done().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        println!("Waiting for all workers to complete...");
        wait_group.wait().await;
        println!("All workers completed!");

        // 5. Mutex for shared state
        println!("\n--- Mutex for Shared State ---");
        let counter = Arc::new(sync::Mutex::new(0));

        let mut increment_tasks = Vec::new();
        for i in 1..=5 {
            let task = spawn({
                let counter = counter.clone();
                async move {
                    for _ in 0..10 {
                        let mut guard = counter.lock().await;
                        *guard += 1;
                        let current = *guard;
                        drop(guard); // Release lock early
                        println!("Task {} incremented counter to {}", i, current);
                        time::sleep_secs(0.1).await;
                    }
                    Ok::<(), Box<dyn std::error::Error>>(())
                }
            });
            increment_tasks.push(task);
        }

        // Wait for all increment tasks
        for (i, task) in increment_tasks.into_iter().enumerate() {
            task.await??;
        }

        let final_value = *counter.lock().await;
        println!("Final counter value: {}", final_value);

        // 6. Condition variable
        println!("\n--- Condition Variable ---");
        let condition = sync::Condition::new();
        let ready = Arc::new(sync::Mutex::new(false));

        let waiter_task = spawn({
            let condition = condition.clone();
            let ready = ready.clone();
            async move {
                println!("Waiter: Waiting for signal...");
                loop {
                    condition.wait().await;
                    let is_ready = *ready.lock().await;
                    if is_ready {
                        println!("Waiter: Received signal and condition is ready!");
                        break;
                    }
                    println!("Waiter: False alarm, continuing to wait...");
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        time::sleep_secs(1.0).await;

        // Send false signal first
        println!("Signaling without setting ready flag...");
        condition.notify_one();

        time::sleep_secs(1.0).await;

        // Now set ready and signal
        {
            let mut ready_guard = ready.lock().await;
            *ready_guard = true;
        }
        println!("Setting ready flag and signaling...");
        condition.notify_one();

        waiter_task.await??;

        println!("\nChannels and synchronization demo completed!");

        Ok::<(), Box<dyn std::error::Error>>(())
    }).await?;

    Ok(())
}