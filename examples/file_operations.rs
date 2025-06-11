use rust_eco::{file, spawn, Eco};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eco = Eco::new();

    eco.run(async {
        println!("File operations demo");

        let temp_dir = "/tmp/rust-eco-demo";
        let file1 = format!("{}/file1.txt", temp_dir);
        let file2 = format!("{}/file2.txt", temp_dir);

        // Create directory
        println!("Creating directory: {}", temp_dir);
        file::mkdir(temp_dir).await?;

        // Spawn concurrent file operations
        let write_task1 = spawn({
            let file1 = file1.clone();
            async move {
                println!("Writing to file1...");
                let content = "Hello from rust-eco!\nThis is file 1.";
                file::write_string(&file1, content).await?;
                println!("File1 written successfully");
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        let write_task2 = spawn({
            let file2 = file2.clone();
            async move {
                println!("Writing to file2...");
                let content = "Greetings from rust-eco!\nThis is file 2.";
                file::write_string(&file2, content).await?;
                println!("File2 written successfully");
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        // Wait for writes to complete
        tokio::try_join!(write_task1, write_task2)?;

        // Read files back
        let read_task1 = spawn({
            let file1 = file1.clone();
            async move {
                println!("Reading file1...");
                let content = file::read_to_string(&file1).await?;
                println!("File1 content ({} bytes):\n{}", content.len(), content);
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        let read_task2 = spawn({
            let file2 = file2.clone();
            async move {
                println!("Reading file2...");
                let content = file::read_to_string(&file2).await?;
                println!("File2 content ({} bytes):\n{}", content.len(), content);
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        tokio::try_join!(read_task1, read_task2)?;

        // List directory contents
        println!("\nDirectory listing:");
        let entries = file::list_dir(temp_dir).await?;
        for entry in entries {
            let info = file::stat(&entry).await?;
            println!("  {} ({} bytes, {})", 
                entry.display(), 
                info.size,
                if info.is_file { "file" } else { "directory" }
            );
        }

        // Append to file1
        println!("\nAppending to file1...");
        file::append(&file1, b"\nAppended line!").await?;
        
        let updated_content = file::read_to_string(&file1).await?;
        println!("Updated file1 content:\n{}", updated_content);

        // Copy file
        let file1_copy = format!("{}/file1_copy.txt", temp_dir);
        println!("\nCopying file1 to file1_copy...");
        file::copy(&file1, &file1_copy).await?;

        // Walk directory
        println!("\nWalking directory tree:");
        file::walk_dir(temp_dir, |path| {
            println!("  Found: {}", path.display());
            true // continue walking
        }).await?;

        // Cleanup
        println!("\nCleaning up...");
        file::remove_dir(temp_dir).await?;
        println!("Demo completed successfully!");

        Ok::<(), Box<dyn std::error::Error>>(())
    }).await?;

    Ok(())
}