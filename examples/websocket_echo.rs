use rust_eco::{protocols::{WebSocketServer, WebSocketClient, WebSocketMessage}, spawn, time, Eco};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eco = Eco::new();

    eco.run(async {
        println!("WebSocket echo server demo");

        // Start WebSocket server
        let server = WebSocketServer::bind("127.0.0.1:0").await?;
        let server_addr = server.local_addr()?;
        println!("WebSocket server listening on: ws://{}", server_addr);

        // Spawn server task
        let server_task = spawn(async move {
            server.serve(|mut connection| async move {
                let peer_addr = connection.peer_addr();
                println!("WebSocket connection from: {}", peer_addr);

                // Send welcome message
                connection.send_text("Welcome to rust-eco WebSocket echo server!").await?;

                // Echo server - receive and echo back messages
                while let Some(message) = connection.recv().await {
                    match &message {
                        WebSocketMessage::Text(text) => {
                            println!("Received text from {}: {}", peer_addr, text);
                            let echo_text = format!("Echo: {}", text);
                            connection.send_text(&echo_text).await?;
                        }
                        WebSocketMessage::Binary(data) => {
                            println!("Received binary from {} ({} bytes)", peer_addr, data.len());
                            let mut echo_data = b"Echo: ".to_vec();
                            echo_data.extend_from_slice(data);
                            connection.send_binary(&echo_data).await?;
                        }
                        WebSocketMessage::Ping(data) => {
                            println!("Received ping from {}", peer_addr);
                            connection.send(WebSocketMessage::Pong(data.clone())).await?;
                        }
                        WebSocketMessage::Pong(_) => {
                            println!("Received pong from {}", peer_addr);
                        }
                        WebSocketMessage::Close => {
                            println!("Connection closed by client: {}", peer_addr);
                            break;
                        }
                    }
                }

                println!("WebSocket connection handler finished for: {}", peer_addr);
                Ok(())
            }).await?;

            Ok::<(), Box<dyn std::error::Error>>(())
        });

        // Give server time to start
        time::sleep_secs(0.1).await;

        // Spawn client tasks
        let mut client_tasks = Vec::new();

        for i in 1..=2 {
            let client_task = spawn({
                let server_addr = server_addr;
                async move {
                    println!("Client {} connecting to WebSocket server...", i);
                    let url = format!("ws://{}", server_addr);
                    let mut client = WebSocketClient::connect(&url).await?;

                    // Wait for welcome message
                    if let Some(WebSocketMessage::Text(welcome)) = client.recv().await {
                        println!("Client {} received welcome: {}", i, welcome);
                    }

                    // Send text messages
                    for j in 1..=3 {
                        let message = format!("Hello from client {} - message {}", i, j);
                        client.send_text(&message).await?;
                        
                        // Wait for echo response
                        if let Some(WebSocketMessage::Text(response)) = client.recv().await {
                            println!("Client {} received: {}", i, response);
                        }

                        time::sleep_secs(0.3).await;
                    }

                    // Send binary message
                    let binary_data = format!("Binary data from client {}", i).into_bytes();
                    client.send_binary(&binary_data).await?;
                    
                    if let Some(WebSocketMessage::Binary(response)) = client.recv().await {
                        let response_text = String::from_utf8_lossy(&response);
                        println!("Client {} received binary echo: {}", i, response_text);
                    }

                    // Send ping
                    let ping_data = format!("ping-{}", i).into_bytes();
                    client.send(WebSocketMessage::Ping(ping_data)).await?;

                    // Wait for pong
                    if let Some(WebSocketMessage::Pong(pong_data)) = client.recv().await {
                        let pong_text = String::from_utf8_lossy(&pong_data);
                        println!("Client {} received pong: {}", i, pong_text);
                    }

                    // Close connection
                    client.close().await?;
                    println!("Client {} disconnected", i);

                    Ok::<(), Box<dyn std::error::Error>>(())
                }
            });

            client_tasks.push(client_task);
        }

        // Wait for all clients to complete
        println!("Waiting for all WebSocket clients to complete...");
        for (i, task) in client_tasks.into_iter().enumerate() {
            match task.await {
                Ok(Ok(())) => println!("WebSocket client {} completed successfully", i + 1),
                Ok(Err(e)) => println!("WebSocket client {} error: {}", i + 1, e),
                Err(e) => println!("WebSocket client {} task panicked: {}", i + 1, e),
            }
        }

        // Let server handle remaining connections
        time::sleep_secs(0.5).await;

        println!("WebSocket demo completed!");

        Ok::<(), Box<dyn std::error::Error>>(())
    }).await?;

    Ok(())
}