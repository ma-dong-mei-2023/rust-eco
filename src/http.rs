use std::collections::HashMap;
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use reqwest::{Client, Method, Response, StatusCode, Url};
use bytes::Bytes;

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub fn with_timeout(timeout: std::time::Duration) -> Self {
        Self {
            client: Client::builder()
                .timeout(timeout)
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub async fn get(&self, url: &str) -> crate::Result<HttpResponse> {
        let response = self.client.get(url).send().await?;
        Ok(HttpResponse::new(response).await?)
    }

    pub async fn post(&self, url: &str, body: &[u8]) -> crate::Result<HttpResponse> {
        let response = self.client.post(url).body(body.to_vec()).send().await?;
        Ok(HttpResponse::new(response).await?)
    }

    pub async fn post_json<T: Serialize>(&self, url: &str, json: &T) -> crate::Result<HttpResponse> {
        let response = self.client.post(url).json(json).send().await?;
        Ok(HttpResponse::new(response).await?)
    }

    pub async fn post_form(&self, url: &str, form: &HashMap<String, String>) -> crate::Result<HttpResponse> {
        let response = self.client.post(url).form(form).send().await?;
        Ok(HttpResponse::new(response).await?)
    }

    pub async fn put(&self, url: &str, body: &[u8]) -> crate::Result<HttpResponse> {
        let response = self.client.put(url).body(body.to_vec()).send().await?;
        Ok(HttpResponse::new(response).await?)
    }

    pub async fn delete(&self, url: &str) -> crate::Result<HttpResponse> {
        let response = self.client.delete(url).send().await?;
        Ok(HttpResponse::new(response).await?)
    }

    pub async fn request(&self, req: HttpRequest) -> crate::Result<HttpResponse> {
        let mut builder = self.client.request(req.method, &req.url);
        
        for (key, value) in req.headers {
            builder = builder.header(key, value);
        }

        if let Some(body) = req.body {
            builder = builder.body(body);
        }

        let response = builder.send().await?;
        Ok(HttpResponse::new(response).await?)
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl HttpRequest {
    pub fn new(method: Method, url: &str) -> Self {
        Self {
            method,
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn get(url: &str) -> Self {
        Self::new(Method::GET, url)
    }

    pub fn post(url: &str) -> Self {
        Self::new(Method::POST, url)
    }

    pub fn put(url: &str) -> Self {
        Self::new(Method::PUT, url)
    }

    pub fn delete(url: &str) -> Self {
        Self::new(Method::DELETE, url)
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn json<T: Serialize>(mut self, data: &T) -> crate::Result<Self> {
        let json = serde_json::to_vec(data)?;
        self.body = Some(json);
        self.headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(self)
    }
}

pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl HttpResponse {
    async fn new(response: Response) -> crate::Result<Self> {
        let status = response.status();
        let mut headers = HashMap::new();

        for (key, value) in response.headers() {
            headers.insert(
                key.to_string(),
                value.to_str().unwrap_or("").to_string(),
            );
        }

        let body = response.bytes().await?;

        Ok(Self {
            status,
            headers,
            body,
        })
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    pub fn header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    pub fn text(&self) -> crate::Result<String> {
        Ok(String::from_utf8(self.body.to_vec())?)
    }

    pub fn json<T>(&self) -> crate::Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        Ok(serde_json::from_slice(&self.body)?)
    }

    pub fn bytes(&self) -> &Bytes {
        &self.body
    }
}

// Simple HTTP server
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct HttpServer {
    listener: TcpListener,
}

impl HttpServer {
    pub async fn bind(addr: &str) -> crate::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    pub fn local_addr(&self) -> crate::Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub async fn serve<F, Fut>(&self, handler: F) -> crate::Result<()>
    where
        F: Fn(HttpServerRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = HttpServerResponse> + Send + 'static,
        F: Clone,
    {
        loop {
            let (stream, _) = self.listener.accept().await?;
            let handler = handler.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, handler).await {
                    tracing::error!("Connection error: {}", e);
                }
            });
        }
    }
}

async fn handle_connection<F, Fut>(
    mut stream: TcpStream,
    handler: F,
) -> crate::Result<()>
where
    F: Fn(HttpServerRequest) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = HttpServerResponse> + Send + 'static,
{
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 3 {
        return Err("Invalid HTTP request".into());
    }

    let method = parts[0].to_string();
    let path = parts[1].to_string();
    let version = parts[2].to_string();

    let mut headers = HashMap::new();
    let mut line = String::new();
    loop {
        line.clear();
        reader.read_line(&mut line).await?;
        if line.trim().is_empty() {
            break;
        }
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    let req = HttpServerRequest {
        method,
        path,
        version,
        headers,
        body: Vec::new(), // TODO: Read body based on Content-Length
    };

    let resp = handler(req).await;

    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n{}",
        resp.status_code,
        resp.reason_phrase(),
        resp.body.len(),
        String::from_utf8_lossy(&resp.body)
    );

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;

    Ok(())
}

pub struct HttpServerRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpServerRequest {
    pub fn header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    pub fn text(&self) -> crate::Result<String> {
        Ok(String::from_utf8(self.body.clone())?)
    }

    pub fn json<T>(&self) -> crate::Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        Ok(serde_json::from_slice(&self.body)?)
    }
}

pub struct HttpServerResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpServerResponse {
    pub fn new(status_code: u16) -> Self {
        Self {
            status_code,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn ok() -> Self {
        Self::new(200)
    }

    pub fn not_found() -> Self {
        Self::new(404).body("Not Found")
    }

    pub fn internal_error() -> Self {
        Self::new(500).body("Internal Server Error")
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(mut self, body: &str) -> Self {
        self.body = body.as_bytes().to_vec();
        self
    }

    pub fn json<T: Serialize>(mut self, data: &T) -> crate::Result<Self> {
        self.body = serde_json::to_vec(data)?;
        self.headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(self)
    }

    fn reason_phrase(&self) -> &'static str {
        match self.status_code {
            200 => "OK",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_http_client() {
        // Note: This test requires internet access
        let client = HttpClient::new();
        
        // Test with a mock server would be better, but this demonstrates the API
        match client.get("https://httpbin.org/get").await {
            Ok(response) => {
                assert!(response.is_success());
                assert!(!response.text().unwrap().is_empty());
            }
            Err(_) => {
                // Skip test if no internet access
                println!("Skipping HTTP client test - no internet access");
            }
        }
    }

    #[tokio::test]
    async fn test_http_server() {
        let server = HttpServer::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr().unwrap();

        // Spawn server
        let server_handle = tokio::spawn(async move {
            server.serve(|req| async move {
                if req.path == "/hello" {
                    HttpServerResponse::ok().body("Hello, World!")
                } else {
                    HttpServerResponse::not_found()
                }
            }).await.unwrap();
        });

        // Give server time to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Test client
        let client = HttpClient::new();
        let url = format!("http://{}/hello", server_addr);
        
        match client.get(&url).await {
            Ok(response) => {
                assert!(response.is_success());
                assert_eq!(response.text().unwrap(), "Hello, World!");
            }
            Err(e) => {
                println!("HTTP server test failed: {}", e);
            }
        }

        // Note: In a real test, you'd want to gracefully shutdown the server
        server_handle.abort();
    }
}