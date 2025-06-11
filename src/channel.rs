use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, broadcast, oneshot, Mutex, Notify};
use tokio::time::{timeout, Duration};

pub struct Channel<T> {
    sender: mpsc::UnboundedSender<T>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<T>>>,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn bounded(capacity: usize) -> BoundedChannel<T> {
        let (sender, receiver) = mpsc::channel(capacity);
        BoundedChannel {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(value)
    }

    pub async fn recv(&self) -> Option<T> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        match self.receiver.try_lock() {
            Ok(mut receiver) => match receiver.try_recv() {
                Ok(value) => Ok(value),
                Err(mpsc::error::TryRecvError::Empty) => Err(TryRecvError::Empty),
                Err(mpsc::error::TryRecvError::Disconnected) => Err(TryRecvError::Disconnected),
            },
            Err(_) => Err(TryRecvError::WouldBlock),
        }
    }

    pub fn sender(&self) -> ChannelSender<T> {
        ChannelSender {
            sender: self.sender.clone(),
        }
    }

    pub fn receiver(&self) -> ChannelReceiver<T> {
        ChannelReceiver {
            receiver: self.receiver.clone(),
        }
    }
}

impl<T> Clone for Channel<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BoundedChannel<T> {
    sender: mpsc::Sender<T>,
    receiver: Arc<Mutex<mpsc::Receiver<T>>>,
}

impl<T> BoundedChannel<T> {
    pub async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(value).await
    }

    pub async fn recv(&self) -> Option<T> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }

    pub fn try_send(&self, value: T) -> Result<(), mpsc::error::TrySendError<T>> {
        self.sender.try_send(value)
    }

    pub fn sender(&self) -> BoundedChannelSender<T> {
        BoundedChannelSender {
            sender: self.sender.clone(),
        }
    }

    pub fn receiver(&self) -> BoundedChannelReceiver<T> {
        BoundedChannelReceiver {
            receiver: self.receiver.clone(),
        }
    }
}

impl<T> Clone for BoundedChannel<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

#[derive(Debug)]
pub enum TryRecvError {
    Empty,
    Disconnected,
    WouldBlock,
}

pub struct ChannelSender<T> {
    sender: mpsc::UnboundedSender<T>,
}

impl<T> ChannelSender<T> {
    pub fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(value)
    }
}

impl<T> Clone for ChannelSender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

pub struct ChannelReceiver<T> {
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<T>>>,
}

impl<T> ChannelReceiver<T> {
    pub async fn recv(&self) -> Option<T> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }

    pub async fn recv_timeout(&self, duration: Duration) -> Result<Option<T>, tokio::time::error::Elapsed> {
        timeout(duration, self.recv()).await
    }
}

impl<T> Clone for ChannelReceiver<T> {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
        }
    }
}

pub struct BoundedChannelSender<T> {
    sender: mpsc::Sender<T>,
}

impl<T> BoundedChannelSender<T> {
    pub async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(value).await
    }

    pub fn try_send(&self, value: T) -> Result<(), mpsc::error::TrySendError<T>> {
        self.sender.try_send(value)
    }
}

impl<T> Clone for BoundedChannelSender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

pub struct BoundedChannelReceiver<T> {
    receiver: Arc<Mutex<mpsc::Receiver<T>>>,
}

impl<T> BoundedChannelReceiver<T> {
    pub async fn recv(&self) -> Option<T> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }

    pub async fn recv_timeout(&self, duration: Duration) -> Result<Option<T>, tokio::time::error::Elapsed> {
        timeout(duration, self.recv()).await
    }
}

impl<T> Clone for BoundedChannelReceiver<T> {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
        }
    }
}

pub struct Broadcast<T> {
    sender: broadcast::Sender<T>,
}

impl<T> Broadcast<T>
where
    T: Clone,
{
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn send(&self, value: T) -> Result<usize, broadcast::error::SendError<T>> {
        self.sender.send(value)
    }

    pub fn subscribe(&self) -> BroadcastReceiver<T> {
        BroadcastReceiver {
            receiver: self.sender.subscribe(),
        }
    }

    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl<T> Clone for Broadcast<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

pub struct BroadcastReceiver<T> {
    receiver: broadcast::Receiver<T>,
}

impl<T> BroadcastReceiver<T>
where
    T: Clone,
{
    pub async fn recv(&mut self) -> Result<T, broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    pub fn try_recv(&mut self) -> Result<T, broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

pub struct OneShot<T> {
    sender: Option<oneshot::Sender<T>>,
    receiver: Option<oneshot::Receiver<T>>,
}

impl<T> OneShot<T> {
    pub fn new() -> Self {
        let (sender, receiver) = oneshot::channel();
        Self {
            sender: Some(sender),
            receiver: Some(receiver),
        }
    }

    pub fn send(mut self, value: T) -> Result<(), T> {
        if let Some(sender) = self.sender.take() {
            sender.send(value)
        } else {
            Err(value)
        }
    }

    pub async fn recv(mut self) -> Result<T, oneshot::error::RecvError> {
        if let Some(receiver) = self.receiver.take() {
            receiver.await
        } else {
            // Create a closed channel to get a proper RecvError
            let (_, rx) = oneshot::channel::<T>();
            rx.await
        }
    }

    pub fn split(mut self) -> (OneShotSender<T>, OneShotReceiver<T>) {
        let sender = OneShotSender {
            sender: self.sender.take(),
        };
        let receiver = OneShotReceiver {
            receiver: self.receiver.take(),
        };
        (sender, receiver)
    }
}

impl<T> Default for OneShot<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OneShotSender<T> {
    sender: Option<oneshot::Sender<T>>,
}

impl<T> OneShotSender<T> {
    pub fn send(mut self, value: T) -> Result<(), T> {
        if let Some(sender) = self.sender.take() {
            sender.send(value)
        } else {
            Err(value)
        }
    }
}

pub struct OneShotReceiver<T> {
    receiver: Option<oneshot::Receiver<T>>,
}

impl<T> OneShotReceiver<T> {
    pub async fn recv(mut self) -> Result<T, oneshot::error::RecvError> {
        if let Some(receiver) = self.receiver.take() {
            receiver.await
        } else {
            // Create a closed channel to get a proper RecvError
            let (_, rx) = oneshot::channel::<T>();
            rx.await
        }
    }

    pub async fn recv_timeout(self, duration: Duration) -> Result<T, TimeoutOrRecvError> {
        match timeout(duration, self.recv()).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => Err(TimeoutOrRecvError::Recv(e)),
            Err(e) => Err(TimeoutOrRecvError::Timeout(e)),
        }
    }
}

#[derive(Debug)]
pub enum TimeoutOrRecvError {
    Timeout(tokio::time::error::Elapsed),
    Recv(oneshot::error::RecvError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_unbounded_channel() {
        let channel = Channel::new();
        let sender = channel.sender();
        let receiver = channel.receiver();

        sender.send(42).unwrap();
        sender.send(43).unwrap();

        assert_eq!(receiver.recv().await, Some(42));
        assert_eq!(receiver.recv().await, Some(43));
    }

    #[tokio::test]
    async fn test_bounded_channel() {
        let channel = Channel::bounded(2);
        let sender = channel.sender();
        let receiver = channel.receiver();

        sender.send(1).await.unwrap();
        sender.send(2).await.unwrap();

        // Channel should be full
        assert!(sender.try_send(3).is_err());

        assert_eq!(receiver.recv().await, Some(1));
        assert_eq!(receiver.recv().await, Some(2));

        // Now we can send again
        sender.send(3).await.unwrap();
        assert_eq!(receiver.recv().await, Some(3));
    }

    #[tokio::test]
    async fn test_broadcast() {
        let broadcast = Broadcast::new(16);
        let mut receiver1 = broadcast.subscribe();
        let mut receiver2 = broadcast.subscribe();

        broadcast.send(42).unwrap();

        assert_eq!(receiver1.recv().await.unwrap(), 42);
        assert_eq!(receiver2.recv().await.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_oneshot() {
        let oneshot = OneShot::new();
        let (sender, receiver) = oneshot.split();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            sender.send(42).unwrap();
        });

        let value = receiver.recv().await.unwrap();
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn test_oneshot_timeout() {
        let oneshot = OneShot::new();
        let (sender, receiver) = oneshot.split();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            sender.send(42).unwrap();
        });

        let result = receiver.recv_timeout(Duration::from_millis(50)).await;
        assert!(matches!(result, Err(TimeoutOrRecvError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_channel_try_recv() {
        let channel = Channel::new();
        let sender = channel.sender();

        // Channel is empty
        assert!(matches!(channel.try_recv(), Err(TryRecvError::Empty)));

        sender.send(42).unwrap();

        // Now we have a value
        assert_eq!(channel.try_recv().unwrap(), 42);

        // Channel is empty again
        assert!(matches!(channel.try_recv(), Err(TryRecvError::Empty)));
    }
}