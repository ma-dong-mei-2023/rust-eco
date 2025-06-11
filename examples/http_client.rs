use rust_eco::{http, spawn, time, Eco};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eco = Eco::new();

    eco.run(async {
        println!("HTTP client demo");

        let client = http::HttpClient::new();

        // Spawn multiple HTTP requests concurrently
        let get_task = spawn({
            let client = client.clone();
            async move {
                println!("Making GET request to httpbin.org...");
                match client.get("https://httpbin.org/get").await {
                    Ok(response) => {
                        println!("GET Response status: {}", response.status());
                        if response.is_success() {
                            let text = response.text()?;
                            println!("GET Response (first 200 chars): {}", 
                                &text[..text.len().min(200)]);
                        }
                    }
                    Err(e) => println!("GET request failed: {}", e),
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        let post_task = spawn({
            let client = client.clone();
            async move {
                println!("Making POST request with JSON...");
                let json_data = json!({
                    "message": "Hello from rust-eco!",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "version": "0.1.0"
                });

                match client.post_json("https://httpbin.org/post", &json_data).await {
                    Ok(response) => {
                        println!("POST Response status: {}", response.status());
                        if response.is_success() {
                            let text = response.text()?;
                            println!("POST Response (first 300 chars): {}", 
                                &text[..text.len().min(300)]);
                        }
                    }
                    Err(e) => println!("POST request failed: {}", e),
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        let form_task = spawn({
            let client = client.clone();
            async move {
                println!("Making POST request with form data...");
                let mut form_data = HashMap::new();
                form_data.insert("name".to_string(), "rust-eco".to_string());
                form_data.insert("type".to_string(), "async-runtime".to_string());
                form_data.insert("language".to_string(), "Rust".to_string());

                match client.post_form("https://httpbin.org/post", &form_data).await {
                    Ok(response) => {
                        println!("FORM Response status: {}", response.status());
                        if response.is_success() {
                            let text = response.text()?;
                            println!("FORM Response (first 300 chars): {}", 
                                &text[..text.len().min(300)]);
                        }
                    }
                    Err(e) => println!("FORM request failed: {}", e),
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        let custom_request_task = spawn({
            let client = client.clone();
            async move {
                println!("Making custom request with headers...");
                let request = http::HttpRequest::get("https://httpbin.org/headers")
                    .header("User-Agent", "rust-eco/0.1.0")
                    .header("X-Custom-Header", "demo-value");

                match client.request(request).await {
                    Ok(response) => {
                        println!("CUSTOM Response status: {}", response.status());
                        if response.is_success() {
                            let text = response.text()?;
                            println!("CUSTOM Response (first 400 chars): {}", 
                                &text[..text.len().min(400)]);
                        }
                    }
                    Err(e) => println!("CUSTOM request failed: {}", e),
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        });

        // Wait for all requests to complete
        println!("Waiting for all HTTP requests to complete...");
        tokio::try_join!(get_task, post_task, form_task, custom_request_task)?;

        println!("\nHTTP client demo completed!");

        Ok::<(), Box<dyn std::error::Error>>(())
    }).await?;

    Ok(())
}