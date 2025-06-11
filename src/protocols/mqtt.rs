use rumqttc::{Client, Connection, Event, MqttOptions, Packet, Publish, QoS as RumqttcQoS};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QoS {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl From<QoS> for RumqttcQoS {
    fn from(qos: QoS) -> Self {
        match qos {
            QoS::AtMostOnce => RumqttcQoS::AtMostOnce,
            QoS::AtLeastOnce => RumqttcQoS::AtLeastOnce,
            QoS::ExactlyOnce => RumqttcQoS::ExactlyOnce,
        }
    }
}

impl From<RumqttcQoS> for QoS {
    fn from(qos: RumqttcQoS) -> Self {
        match qos {
            RumqttcQoS::AtMostOnce => QoS::AtMostOnce,
            RumqttcQoS::AtLeastOnce => QoS::AtLeastOnce,
            RumqttcQoS::ExactlyOnce => QoS::ExactlyOnce,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: QoS,
    pub retain: bool,
}

impl MqttMessage {
    pub fn new(topic: &str, payload: Vec<u8>) -> Self {
        Self {
            topic: topic.to_string(),
            payload,
            qos: QoS::AtMostOnce,
            retain: false,
        }
    }

    pub fn with_qos(mut self, qos: QoS) -> Self {
        self.qos = qos;
        self
    }

    pub fn with_retain(mut self, retain: bool) -> Self {
        self.retain = retain;
        self
    }

    pub fn payload_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.payload.clone())
    }
}

pub struct MqttClient {
    client: Client,
    connection: Connection,
    message_rx: mpsc::UnboundedReceiver<MqttMessage>,
}

impl MqttClient {
    pub async fn connect(
        client_id: &str,
        host: &str,
        port: u16,
    ) -> crate::Result<Self> {
        Self::connect_with_options(client_id, host, port, None, None).await
    }

    pub async fn connect_with_auth(
        client_id: &str,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> crate::Result<Self> {
        Self::connect_with_options(
            client_id,
            host,
            port,
            Some(username.to_string()),
            Some(password.to_string()),
        ).await
    }

    async fn connect_with_options(
        client_id: &str,
        host: &str,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    ) -> crate::Result<Self> {
        let mut mqtt_options = MqttOptions::new(client_id, host, port);
        mqtt_options.set_keep_alive(Duration::from_secs(60));

        if let (Some(user), Some(pass)) = (username, password) {
            mqtt_options.set_credentials(user, pass);
        }

        let (client, mut connection) = Client::new(mqtt_options, 10);
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        // Spawn task to handle connection events
        let message_tx_clone = message_tx.clone();
        tokio::spawn(async move {
            loop {
                match connection.poll().await {
                    Ok(Event::Incoming(Packet::Publish(publish))) => {
                        let message = MqttMessage {
                            topic: publish.topic,
                            payload: publish.payload.to_vec(),
                            qos: publish.qos.into(),
                            retain: publish.retain,
                        };
                        
                        if message_tx_clone.send(message).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {
                        // Handle other events if needed
                    }
                    Err(e) => {
                        tracing::error!("MQTT connection error: {}", e);
                        break;
                    }
                }
            }
        });

        // Wait a bit for connection to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(Self {
            client,
            connection,
            message_rx,
        })
    }

    pub async fn publish(&self, message: &MqttMessage) -> crate::Result<()> {
        self.client
            .publish(
                &message.topic,
                message.qos.into(),
                message.retain,
                message.payload.clone(),
            )
            .await
            .map_err(|e| format!("Failed to publish message: {}", e))?;
        
        Ok(())
    }

    pub async fn publish_string(
        &self,
        topic: &str,
        payload: &str,
        qos: QoS,
        retain: bool,
    ) -> crate::Result<()> {
        let message = MqttMessage {
            topic: topic.to_string(),
            payload: payload.as_bytes().to_vec(),
            qos,
            retain,
        };
        
        self.publish(&message).await
    }

    pub async fn subscribe(&self, topic: &str, qos: QoS) -> crate::Result<()> {
        self.client
            .subscribe(topic, qos.into())
            .await
            .map_err(|e| format!("Failed to subscribe to topic: {}", e))?;
        
        Ok(())
    }

    pub async fn unsubscribe(&self, topic: &str) -> crate::Result<()> {
        self.client
            .unsubscribe(topic)
            .await
            .map_err(|e| format!("Failed to unsubscribe from topic: {}", e))?;
        
        Ok(())
    }

    pub async fn recv(&mut self) -> Option<MqttMessage> {
        self.message_rx.recv().await
    }

    pub async fn recv_timeout(&mut self, duration: Duration) -> Option<MqttMessage> {
        timeout(duration, self.message_rx.recv()).await.ok().flatten()
    }

    pub async fn disconnect(self) -> crate::Result<()> {
        self.client
            .disconnect()
            .await
            .map_err(|e| format!("Failed to disconnect: {}", e))?;
        
        Ok(())
    }
}

// MQTT broker simulation for testing
#[cfg(test)]
pub struct MockMqttBroker {
    port: u16,
    _handle: tokio::task::JoinHandle<()>,
}

#[cfg(test)]
impl MockMqttBroker {
    pub async fn start() -> crate::Result<Self> {
        use tokio::net::TcpListener;
        use std::net::SocketAddr;

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();

        let handle = tokio::spawn(async move {
            // Very basic mock broker - just accept connections
            while let Ok((socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    // Echo back data for testing
                    let mut socket = socket;
                    let mut buf = [0; 1024];
                    while let Ok(n) = socket.try_read(&mut buf) {
                        if n == 0 {
                            break;
                        }
                        let _ = socket.try_write(&buf[..n]);
                    }
                });
            }
        });

        Ok(Self {
            port,
            _handle: handle,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_message_creation() {
        let message = MqttMessage::new("test/topic", b"hello".to_vec())
            .with_qos(QoS::AtLeastOnce)
            .with_retain(true);

        assert_eq!(message.topic, "test/topic");
        assert_eq!(message.payload, b"hello");
        assert_eq!(message.qos, QoS::AtLeastOnce);
        assert!(message.retain);
        assert_eq!(message.payload_string().unwrap(), "hello");
    }

    #[test]
    fn test_qos_conversion() {
        assert_eq!(RumqttcQoS::from(QoS::AtMostOnce), RumqttcQoS::AtMostOnce);
        assert_eq!(RumqttcQoS::from(QoS::AtLeastOnce), RumqttcQoS::AtLeastOnce);
        assert_eq!(RumqttcQoS::from(QoS::ExactlyOnce), RumqttcQoS::ExactlyOnce);

        assert_eq!(QoS::from(RumqttcQoS::AtMostOnce), QoS::AtMostOnce);
        assert_eq!(QoS::from(RumqttcQoS::AtLeastOnce), QoS::AtLeastOnce);
        assert_eq!(QoS::from(RumqttcQoS::ExactlyOnce), QoS::ExactlyOnce);
    }

    // Note: Integration tests would require a real MQTT broker
    // For now, we'll focus on unit tests for the data structures
}