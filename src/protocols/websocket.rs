use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    accept_async, connect_async, tungstenite::Message as TungsteniteMessage,
    WebSocketStream, MaybeTlsStream,
};
use futures::{SinkExt, StreamExt};
use url::Url;

#[derive(Debug, Clone)]
pub enum WebSocketMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}

impl From<TungsteniteMessage> for WebSocketMessage {
    fn from(msg: TungsteniteMessage) -> Self {
        match msg {
            TungsteniteMessage::Text(text) => WebSocketMessage::Text(text),
            TungsteniteMessage::Binary(data) => WebSocketMessage::Binary(data),
            TungsteniteMessage::Ping(data) => WebSocketMessage::Ping(data),
            TungsteniteMessage::Pong(data) => WebSocketMessage::Pong(data),
            TungsteniteMessage::Close(_) => WebSocketMessage::Close,
            TungsteniteMessage::Frame(_) => WebSocketMessage::Close, // Shouldn't happen in normal usage
        }
    }
}

impl From<WebSocketMessage> for TungsteniteMessage {
    fn from(msg: WebSocketMessage) -> Self {
        match msg {
            WebSocketMessage::Text(text) => TungsteniteMessage::Text(text),
            WebSocketMessage::Binary(data) => TungsteniteMessage::Binary(data),
            WebSocketMessage::Ping(data) => TungsteniteMessage::Ping(data),
            WebSocketMessage::Pong(data) => TungsteniteMessage::Pong(data),
            WebSocketMessage::Close => TungsteniteMessage::Close(None),
        }
    }
}

pub struct WebSocketClient {
    sender: mpsc::UnboundedSender<WebSocketMessage>,
    receiver: mpsc::UnboundedReceiver<WebSocketMessage>,
    _connection_handle: tokio::task::JoinHandle<()>,
}

impl WebSocketClient {
    pub async fn connect(url: &str) -> crate::Result<Self> {
        Self::connect_with_headers(url, HashMap::new()).await
    }

    pub async fn connect_with_headers(
        url: &str,
        headers: HashMap<String, String>,
    ) -> crate::Result<Self> {
        let url = Url::parse(url)
            .map_err(|e| format!("Invalid WebSocket URL: {}", e))?;

        let mut request = tungstenite::handshake::client::Request::builder()
            .uri(url.as_str());

        for (key, value) in headers {
            request = request.header(key, value);
        }

        let request = request.body(())
            .map_err(|e| format!("Failed to build request: {}", e))?;

        let (ws_stream, _) = connect_async(request).await
            .map_err(|e| format!("WebSocket connection failed: {}", e))?;

        Self::from_stream(ws_stream).await
    }

    async fn from_stream(
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> crate::Result<Self> {
        let (mut sink, mut stream) = ws_stream.split();
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (send_tx, mut send_rx) = mpsc::unbounded_channel();

        // Spawn task to handle outgoing messages
        let sink_handle = tokio::spawn(async move {
            while let Some(msg) = send_rx.recv().await {
                let tungstenite_msg: TungsteniteMessage = msg.into();
                if sink.send(tungstenite_msg).await.is_err() {
                    break;
                }
            }
        });

        // Spawn task to handle incoming messages
        let stream_handle = tokio::spawn(async move {
            while let Some(msg_result) = stream.next().await {
                match msg_result {
                    Ok(msg) => {
                        let ws_msg: WebSocketMessage = msg.into();
                        if msg_tx.send(ws_msg).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Combined handle that waits for either task to complete
        let connection_handle = tokio::spawn(async move {
            tokio::select! {
                _ = sink_handle => {}
                _ = stream_handle => {}
            }
        });

        Ok(Self {
            sender: send_tx,
            receiver: msg_rx,
            _connection_handle: connection_handle,
        })
    }

    pub async fn send(&self, message: WebSocketMessage) -> crate::Result<()> {
        self.sender.send(message)
            .map_err(|_| "WebSocket connection closed")?;
        Ok(())
    }

    pub async fn send_text(&self, text: &str) -> crate::Result<()> {
        self.send(WebSocketMessage::Text(text.to_string())).await
    }

    pub async fn send_binary(&self, data: &[u8]) -> crate::Result<()> {
        self.send(WebSocketMessage::Binary(data.to_vec())).await
    }

    pub async fn recv(&mut self) -> Option<WebSocketMessage> {
        self.receiver.recv().await
    }

    pub async fn close(&self) -> crate::Result<()> {
        self.send(WebSocketMessage::Close).await
    }
}

pub struct WebSocketServer {
    listener: TcpListener,
}

impl WebSocketServer {
    pub async fn bind(addr: &str) -> crate::Result<Self> {
        let listener = TcpListener::bind(addr).await
            .map_err(|e| format!("Failed to bind WebSocket server: {}", e))?;
        
        Ok(Self { listener })
    }

    pub fn local_addr(&self) -> crate::Result<SocketAddr> {
        self.listener.local_addr()
            .map_err(|e| format!("Failed to get local address: {}", e).into())
    }

    pub async fn accept(&self) -> crate::Result<WebSocketConnection> {
        let (stream, addr) = self.listener.accept().await
            .map_err(|e| format!("Failed to accept connection: {}", e))?;

        let ws_stream = accept_async(stream).await
            .map_err(|e| format!("WebSocket handshake failed: {}", e))?;

        WebSocketConnection::new(ws_stream, addr).await
    }

    pub async fn serve<F, Fut>(&self, handler: F) -> crate::Result<()>
    where
        F: Fn(WebSocketConnection) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::Result<()>> + Send + 'static,
        F: Clone,
    {
        loop {
            match self.accept().await {
                Ok(connection) => {
                    let handler = handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handler(connection).await {
                            tracing::error!("WebSocket handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept WebSocket connection: {}", e);
                }
            }
        }
    }
}

pub struct WebSocketConnection {
    addr: SocketAddr,
    sender: mpsc::UnboundedSender<WebSocketMessage>,
    receiver: mpsc::UnboundedReceiver<WebSocketMessage>,
    _connection_handle: tokio::task::JoinHandle<()>,
}

impl WebSocketConnection {
    async fn new(
        ws_stream: WebSocketStream<TcpStream>,
        addr: SocketAddr,
    ) -> crate::Result<Self> {
        let (mut sink, mut stream) = ws_stream.split();
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (send_tx, mut send_rx) = mpsc::unbounded_channel();

        // Spawn task to handle outgoing messages
        let sink_handle = tokio::spawn(async move {
            while let Some(msg) = send_rx.recv().await {
                let tungstenite_msg: TungsteniteMessage = msg.into();
                if sink.send(tungstenite_msg).await.is_err() {
                    break;
                }
            }
        });

        // Spawn task to handle incoming messages
        let stream_handle = tokio::spawn(async move {
            while let Some(msg_result) = stream.next().await {
                match msg_result {
                    Ok(msg) => {
                        let ws_msg: WebSocketMessage = msg.into();
                        if msg_tx.send(ws_msg).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Combined handle
        let connection_handle = tokio::spawn(async move {
            tokio::select! {
                _ = sink_handle => {}
                _ = stream_handle => {}
            }
        });

        Ok(Self {
            addr,
            sender: send_tx,
            receiver: msg_rx,
            _connection_handle: connection_handle,
        })
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn send(&self, message: WebSocketMessage) -> crate::Result<()> {
        self.sender.send(message)
            .map_err(|_| "WebSocket connection closed")?;
        Ok(())
    }

    pub async fn send_text(&self, text: &str) -> crate::Result<()> {
        self.send(WebSocketMessage::Text(text.to_string())).await
    }

    pub async fn send_binary(&self, data: &[u8]) -> crate::Result<()> {
        self.send(WebSocketMessage::Binary(data.to_vec())).await
    }

    pub async fn recv(&mut self) -> Option<WebSocketMessage> {
        self.receiver.recv().await
    }

    pub async fn close(&self) -> crate::Result<()> {
        self.send(WebSocketMessage::Close).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_websocket_message_conversion() {
        let text_msg = WebSocketMessage::Text("hello".to_string());
        let tungstenite_msg: TungsteniteMessage = text_msg.clone().into();
        let converted_back: WebSocketMessage = tungstenite_msg.into();
        
        match (text_msg, converted_back) {
            (WebSocketMessage::Text(a), WebSocketMessage::Text(b)) => {
                assert_eq!(a, b);
            }
            _ => panic!("Message conversion failed"),
        }
    }

    #[tokio::test]
    async fn test_websocket_server_creation() {
        let server = WebSocketServer::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        assert!(addr.port() > 0);
    }

    #[tokio::test]
    async fn test_websocket_client_server_communication() {
        // Start server
        let server = WebSocketServer::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr().unwrap();

        // Spawn server handler
        let server_handle = tokio::spawn(async move {
            if let Ok(mut connection) = server.accept().await {
                // Echo back any received messages
                while let Some(msg) = connection.recv().await {
                    match &msg {
                        WebSocketMessage::Text(text) => {
                            let response = format!("Echo: {}", text);
                            if connection.send_text(&response).await.is_err() {
                                break;
                            }
                        }
                        WebSocketMessage::Close => break,
                        _ => {}
                    }
                }
            }
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect client
        let url = format!("ws://127.0.0.1:{}", server_addr.port());
        let mut client = WebSocketClient::connect(&url).await.unwrap();

        // Send message
        client.send_text("Hello WebSocket!").await.unwrap();

        // Receive echo
        if let Some(WebSocketMessage::Text(response)) = client.recv().await {
            assert_eq!(response, "Echo: Hello WebSocket!");
        } else {
            panic!("Expected text response");
        }

        // Close connection
        client.close().await.unwrap();

        // Wait for server to handle the close
        tokio::time::timeout(Duration::from_secs(1), server_handle).await.ok();
    }

    #[tokio::test]
    async fn test_websocket_binary_message() {
        let binary_data = vec![1, 2, 3, 4, 5];
        let msg = WebSocketMessage::Binary(binary_data.clone());
        
        match msg {
            WebSocketMessage::Binary(data) => {
                assert_eq!(data, binary_data);
            }
            _ => panic!("Expected binary message"),
        }
    }
}