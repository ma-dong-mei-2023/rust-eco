use clap::{Arg, Command};
use rust_eco::{log, Eco};
use std::process;

#[tokio::main]
async fn main() {
    let matches = Command::new("eco")
        .version("0.1.0")
        .about("A Rust async runtime inspired by lua-eco")
        .arg(
            Arg::new("script")
                .help("The script file to execute")
                .value_name("FILE")
                .index(1),
        )
        .arg(
            Arg::new("eval")
                .short('e')
                .long("eval")
                .value_name("CODE")
                .help("Execute code directly")
                .takes_value(true),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging")
                .takes_value(false),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress output")
                .takes_value(false),
        )
        .get_matches();

    // Initialize logging
    let log_level = if matches.is_present("verbose") {
        log::LogLevel::Debug
    } else if matches.is_present("quiet") {
        log::LogLevel::Error
    } else {
        log::LogLevel::Info
    };

    if let Err(e) = log::init_logger_with_level(log_level) {
        eprintln!("Failed to initialize logger: {}", e);
        process::exit(1);
    }

    let eco = Eco::new();

    // Handle different execution modes
    if let Some(code) = matches.value_of("eval") {
        // Execute code directly
        println!("Executing code: {}", code);
        // TODO: Implement code execution
        println!("Code execution not yet implemented");
    } else if let Some(script_path) = matches.value_of("script") {
        // Execute script file
        println!("Executing script: {}", script_path);
        // TODO: Implement script execution
        println!("Script execution not yet implemented");
    } else {
        // Interactive mode or demo
        if let Err(e) = run_demo(&eco).await {
            eprintln!("Demo failed: {}", e);
            process::exit(1);
        }
    }
}

async fn run_demo(eco: &Eco) -> Result<(), Box<dyn std::error::Error>> {
    println!("rust-eco demo - running concurrent tasks");
    
    eco.run(async {
        use rust_eco::{spawn, time};
        use std::time::Duration;

        // Spawn multiple concurrent tasks
        let task1 = spawn(async {
            for i in 1..=5 {
                println!("Task 1 - iteration {}", i);
                time::sleep_secs(1.0).await;
            }
            "Task 1 completed"
        });

        let task2 = spawn(async {
            for i in 1..=3 {
                println!("Task 2 - iteration {}", i);
                time::sleep_secs(1.5).await;
            }
            "Task 2 completed"
        });

        let task3 = spawn(async {
            use rust_eco::sys;
            
            println!("System info:");
            if let Ok(info) = sys::SystemInfo::get().await {
                println!("  OS: {}", info.os);
                println!("  Architecture: {}", info.arch);
                println!("  Hostname: {}", info.hostname);
            }
            
            println!("Process ID: {}", sys::pid());
            "Task 3 completed"
        });

        // File operations demo
        let task4 = spawn(async {
            use rust_eco::file;
            
            let demo_file = "/tmp/rust-eco-demo.txt";
            let content = "Hello from rust-eco!\nThis is a demonstration of async file I/O.";
            
            match file::write_string(demo_file, content).await {
                Ok(_) => {
                    println!("Written demo file: {}", demo_file);
                    match file::read_to_string(demo_file).await {
                        Ok(read_content) => {
                            println!("Read back content: {} bytes", read_content.len());
                        }
                        Err(e) => println!("Failed to read file: {}", e),
                    }
                    
                    // Cleanup
                    let _ = file::remove_file(demo_file).await;
                }
                Err(e) => println!("Failed to write file: {}", e),
            }
            
            "Task 4 completed"
        });

        // Network demo
        let task5 = spawn(async {
            use rust_eco::dns;
            
            match dns::resolve("localhost").await {
                Ok(addresses) => {
                    println!("DNS resolution for localhost:");
                    for addr in addresses {
                        println!("  {}", addr);
                    }
                }
                Err(e) => println!("DNS resolution failed: {}", e),
            }
            
            "Task 5 completed"
        });

        // Wait for all tasks to complete
        let results = tokio::try_join!(task1, task2, task3, task4, task5);
        
        match results {
            Ok((r1, r2, r3, r4, r5)) => {
                println!("\nAll tasks completed:");
                println!("  {}", r1);
                println!("  {}", r2);
                println!("  {}", r3);
                println!("  {}", r4);
                println!("  {}", r5);
            }
            Err(e) => {
                eprintln!("Task execution error: {}", e);
            }
        }

        println!("\nDemo completed successfully!");
    }).map_err(|e| format!("Demo error: {}", e))?;

    Ok(())
}