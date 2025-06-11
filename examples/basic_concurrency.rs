use rust_eco::{spawn, time, Eco};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eco = Eco::new();

    eco.run(async {
        println!("Starting concurrent tasks demo");

        // Spawn multiple tasks that run concurrently
        let task1 = spawn(async {
            for i in 1..=5 {
                println!("Task 1 - step {}", i);
                time::sleep_secs(1.0).await;
            }
            "Task 1 completed"
        });

        let task2 = spawn(async {
            for i in 1..=3 {
                println!("Task 2 - step {}", i);
                time::sleep_secs(1.5).await;
            }
            "Task 2 completed"
        });

        let task3 = spawn(async {
            for i in 1..=4 {
                println!("Task 3 - step {}", i);
                time::sleep_secs(0.8).await;
            }
            "Task 3 completed"
        });

        // Wait for all tasks to complete
        let (result1, result2, result3) = tokio::try_join!(task1, task2, task3)?;
        
        println!("Results: {} | {} | {}", result1, result2, result3);
        println!("All tasks completed!");

        Ok::<(), Box<dyn std::error::Error>>(())
    }).await?;

    Ok(())
}