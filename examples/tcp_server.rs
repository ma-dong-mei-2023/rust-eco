use rust_eco::{socket, spawn, time, Eco};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eco = Eco::new();

    eco.run(async {
        println!("TCP server demo");

        let server = socket::TcpServer::bind("127.0.0.1:0").await?;
        let server_addr = server.local_addr()?;
        println!("Server listening on: {}", server_addr);

        let connection_count = Arc::new(AtomicUsize::new(0));

        // Spawn server task
        let server_task = spawn({
            let connection_count = connection_count.clone();
            async move {
                server.serve(move |mut client, addr| {
                    let connection_count = connection_count.clone();
                    async move {
                        let conn_id = connection_count.fetch_add(1, Ordering::SeqCst) + 1;
                        println!("Connection #{} from {}", conn_id, addr);

                        // Send welcome message
                        let welcome = format!("Welcome to rust-eco TCP server! Connection #{}\n", conn_id);
                        client.write_all(welcome.as_bytes()).await?;

                        // Echo server - read and echo back data
                        let mut buf = [0u8; 1024];
                        loop {
                            match client.read(&mut buf).await {
                                Ok(0) => {
                                    println!("Connection #{} closed by client", conn_id);
                                    break;
                                }
                                Ok(n) => {
                                    let received = String::from_utf8_lossy(&buf[..n]);
                                    println!("Connection #{} received: {}", conn_id, received.trim());
                                    
                                    // Echo back with prefix
                                    let echo = format!("Echo #{}: {}", conn_id, received);
                                    client.write_all(echo.as_bytes()).await?;
                                }
                                Err(e) => {
                                    println!("Connection #{} error: {}", conn_id, e);
                                    break;
                                }
                            }
                        }

                        println!("Connection #{} handler finished", conn_id);
                        Ok::<(), Box<dyn std::error::Error>>(())
                    }
                }).await?;

                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        // Give server time to start
        time::sleep_secs(0.1).await;

        // Spawn multiple client tasks
        let mut client_tasks = Vec::new();

        for i in 1..=3 {
            let client_task = spawn({
                let server_addr = server_addr;
                async move {
                    println!("Client {} connecting to server...", i);
                    let mut client = socket::TcpClient::connect(server_addr).await?;
                    
                    // Read welcome message
                    let mut buf = [0u8; 1024];
                    let n = client.read(&mut buf).await?;
                    let welcome = String::from_utf8_lossy(&buf[..n]);
                    println!("Client {} received welcome: {}", i, welcome.trim());

                    // Send some messages
                    for j in 1..=3 {
                        let message = format!("Message {} from client {}\n", j, i);
                        client.write_all(message.as_bytes()).await?;
                        
                        // Read echo response
                        let n = client.read(&mut buf).await?;
                        let response = String::from_utf8_lossy(&buf[..n]);
                        println!("Client {} received echo: {}", i, response.trim());

                        time::sleep_secs(0.5).await;
                    }

                    // Close connection
                    client.shutdown().await?;
                    println!("Client {} disconnected", i);

                    Ok::<(), Box<dyn std::error::Error>>(())
                }
            });

            client_tasks.push(client_task);
        }

        // Wait for all clients to complete
        println!("Waiting for all clients to complete...");
        for (i, task) in client_tasks.into_iter().enumerate() {
            match task.await {
                Ok(Ok(())) => println!("Client {} completed successfully", i + 1),
                Ok(Err(e)) => println!("Client {} error: {}", i + 1, e),
                Err(e) => println!("Client {} task panicked: {}", i + 1, e),
            }
        }

        // Let server handle remaining connections
        time::sleep_secs(1.0).await;

        println!("TCP server demo completed!");
        println!("Total connections handled: {}", connection_count.load(Ordering::SeqCst));

        // Note: In a real application, you'd want to gracefully shutdown the server
        // For this demo, we'll let it be cancelled when the eco runtime exits

        Ok::<(), Box<dyn std::error::Error>>(())
    }).await?;

    Ok(())
}