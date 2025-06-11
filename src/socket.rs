use std::net::SocketAddr;
use tokio::net::ToSocketAddrs;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, AsyncBufReadExt};
use bytes::Bytes;

pub struct TcpServer {
    listener: TcpListener,
}

impl TcpServer {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> crate::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    pub fn local_addr(&self) -> crate::Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub async fn accept(&self) -> crate::Result<(TcpClient, SocketAddr)> {
        let (stream, addr) = self.listener.accept().await?;
        Ok((TcpClient::new(stream), addr))
    }

    pub async fn serve<F, Fut>(&self, handler: F) -> crate::Result<()>
    where
        F: Fn(TcpClient, SocketAddr) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::Result<()>> + Send + 'static,
        F: Clone,
    {
        loop {
            let (client, addr) = self.accept().await?;
            let handler = handler.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handler(client, addr).await {
                    tracing::error!("Handler error: {}", e);
                }
            });
        }
    }
}

pub struct TcpClient {
    stream: TcpStream,
}

impl TcpClient {
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> crate::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self::new(stream))
    }

    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub fn local_addr(&self) -> crate::Result<SocketAddr> {
        Ok(self.stream.local_addr()?)
    }

    pub fn peer_addr(&self) -> crate::Result<SocketAddr> {
        Ok(self.stream.peer_addr()?)
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        let n = self.stream.read(buf).await?;
        Ok(n)
    }

    pub async fn read_exact(&mut self, buf: &mut [u8]) -> crate::Result<()> {
        self.stream.read_exact(buf).await?;
        Ok(())
    }

    pub async fn read_to_end(&mut self) -> crate::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.stream.read_to_end(&mut buf).await?;
        Ok(buf)
    }

    pub async fn read_line(&mut self) -> crate::Result<String> {
        let mut reader = BufReader::new(&mut self.stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        Ok(line)
    }

    pub async fn write(&mut self, buf: &[u8]) -> crate::Result<usize> {
        let n = self.stream.write(buf).await?;
        Ok(n)
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> crate::Result<()> {
        self.stream.write_all(buf).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> crate::Result<()> {
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> crate::Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }
}

pub struct UdpClient {
    socket: UdpSocket,
}

impl UdpClient {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> crate::Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self { socket })
    }

    pub async fn connect<A: ToSocketAddrs>(&self, addr: A) -> crate::Result<()> {
        self.socket.connect(addr).await?;
        Ok(())
    }

    pub fn local_addr(&self) -> crate::Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    pub fn peer_addr(&self) -> crate::Result<SocketAddr> {
        Ok(self.socket.peer_addr()?)
    }

    pub async fn send(&self, buf: &[u8]) -> crate::Result<usize> {
        let n = self.socket.send(buf).await?;
        Ok(n)
    }

    pub async fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> crate::Result<usize> {
        let n = self.socket.send_to(buf, addr).await?;
        Ok(n)
    }

    pub async fn recv(&self, buf: &mut [u8]) -> crate::Result<usize> {
        let n = self.socket.recv(buf).await?;
        Ok(n)
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> crate::Result<(usize, SocketAddr)> {
        let (n, addr) = self.socket.recv_from(buf).await?;
        Ok((n, addr))
    }
}

#[cfg(unix)]
pub mod unix {
    use tokio::net::{UnixListener, UnixStream};
    use std::path::Path;

    pub struct UnixServer {
        listener: UnixListener,
    }

    impl UnixServer {
        pub async fn bind<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
            let listener = UnixListener::bind(path)?;
            Ok(Self { listener })
        }

        pub async fn accept(&self) -> crate::Result<UnixClient> {
            let (stream, _) = self.listener.accept().await?;
            Ok(UnixClient::new(stream))
        }
    }

    pub struct UnixClient {
        stream: UnixStream,
    }

    impl UnixClient {
        pub async fn connect<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
            let stream = UnixStream::connect(path).await?;
            Ok(Self::new(stream))
        }

        pub fn new(stream: UnixStream) -> Self {
            Self { stream }
        }

        pub async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
            use tokio::io::AsyncReadExt;
            let n = self.stream.read(buf).await?;
            Ok(n)
        }

        pub async fn write(&mut self, buf: &[u8]) -> crate::Result<usize> {
            use tokio::io::AsyncWriteExt;
            let n = self.stream.write(buf).await?;
            Ok(n)
        }

        pub async fn write_all(&mut self, buf: &[u8]) -> crate::Result<()> {
            use tokio::io::AsyncWriteExt;
            self.stream.write_all(buf).await?;
            Ok(())
        }

        pub async fn shutdown(&mut self) -> crate::Result<()> {
            use tokio::io::AsyncWriteExt;
            self.stream.shutdown().await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_tcp_server_client() {
        let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr().unwrap();

        // Spawn server task
        let server_handle = tokio::spawn(async move {
            let (mut client, _) = server.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let n = client.read(&mut buf).await.unwrap();
            let msg = String::from_utf8_lossy(&buf[..n]);
            client.write_all(format!("Echo: {}", msg).as_bytes()).await.unwrap();
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Connect client
        let mut client = TcpClient::connect(server_addr).await.unwrap();
        client.write_all(b"Hello").await.unwrap();
        
        let mut buf = [0u8; 1024];
        let n = client.read(&mut buf).await.unwrap();
        let response = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(response, "Echo: Hello");

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_udp_client() {
        let client1 = UdpClient::bind("127.0.0.1:0").await.unwrap();
        let client2 = UdpClient::bind("127.0.0.1:0").await.unwrap();
        
        let addr1 = client1.local_addr().unwrap();
        let addr2 = client2.local_addr().unwrap();

        // Send from client1 to client2
        client1.send_to(b"Hello UDP", addr2).await.unwrap();

        // Receive on client2
        let mut buf = [0u8; 1024];
        let (n, from_addr) = client2.recv_from(&mut buf).await.unwrap();
        
        assert_eq!(&buf[..n], b"Hello UDP");
        assert_eq!(from_addr, addr1);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_unix_socket() {
        use tempfile::NamedTempFile;
        
        let temp_file = NamedTempFile::new().unwrap();
        let socket_path = temp_file.path();

        let server = unix::UnixServer::bind(socket_path).await.unwrap();

        // Spawn server task
        let server_handle = tokio::spawn(async move {
            let mut client = server.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let n = client.read(&mut buf).await.unwrap();
            client.write_all(&buf[..n]).await.unwrap();
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Connect client
        let mut client = unix::UnixClient::connect(socket_path).await.unwrap();
        client.write_all(b"Unix socket test").await.unwrap();
        
        let mut buf = [0u8; 1024];
        let n = client.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"Unix socket test");

        server_handle.await.unwrap();
    }
}