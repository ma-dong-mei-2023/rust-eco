use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{sleep as tokio_sleep, timeout};
use tracing::{debug, error};

pub struct Eco {
    runtime: tokio::runtime::Runtime,
    shutdown_tx: mpsc::UnboundedSender<()>,
    shutdown_rx: Arc<Mutex<mpsc::UnboundedReceiver<()>>>,
}

impl Eco {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();

        Self {
            runtime,
            shutdown_tx,
            shutdown_rx: Arc::new(Mutex::new(shutdown_rx)),
        }
    }

    pub fn run<F>(&self, future: F) -> crate::Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime.block_on(async {
            let shutdown_rx = self.shutdown_rx.clone();
            let main_task = tokio::spawn(future);
            
            tokio::select! {
                result = main_task => {
                    if let Err(e) = result {
                        error!("Main task panicked: {}", e);
                        return Err(crate::EcoError::Runtime(format!("Task panicked: {}", e)).into());
                    }
                }
                _ = shutdown_rx.lock().await.recv() => {
                    debug!("Shutdown signal received");
                }
            }
            
            Ok(())
        })
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    pub fn count(&self) -> usize {
        // Note: tokio doesn't provide direct access to task count
        // This is a placeholder - could be implemented with custom tracking
        0
    }
}

impl Default for Eco {
    fn default() -> Self {
        Self::new()
    }
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}

pub async fn sleep(duration: Duration) {
    tokio_sleep(duration).await;
}

pub async fn sleep_secs(secs: f64) {
    let duration = Duration::from_secs_f64(secs);
    sleep(duration).await;
}

pub async fn yield_now() {
    tokio::task::yield_now().await;
}

pub fn current() -> tokio::task::Id {
    tokio::task::try_id().unwrap_or_else(|| {
        panic!("Called current() outside of tokio task context")
    })
}

pub struct Watcher {
    waker: Option<Waker>,
    active: bool,
}

impl Watcher {
    pub fn new() -> Self {
        Self {
            waker: None,
            active: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn wait(&mut self) -> WatcherFuture {
        self.active = true;
        WatcherFuture { watcher: self }
    }

    pub fn wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            self.active = false;
            waker.wake();
        }
    }

    pub fn cancel(&mut self) {
        self.active = false;
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

pub struct WatcherFuture<'a> {
    watcher: &'a mut Watcher,
}

impl<'a> Future for WatcherFuture<'a> {
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.watcher.active {
            return Poll::Ready(false);
        }

        self.watcher.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_sleep() {
        let start = Instant::now();
        sleep_secs(0.1).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(90));
        assert!(elapsed < Duration::from_millis(200));
    }

    #[tokio::test]
    async fn test_spawn() {
        let handle = spawn(async { 42 });
        let result = handle.await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_watcher() {
        let mut watcher = Watcher::new();
        assert!(!watcher.is_active());

        let mut watcher2 = Watcher::new();
        tokio::spawn(async move {
            sleep_secs(0.1).await;
            watcher2.wake();
        });

        let result = watcher.wait().await;
        assert!(result);
    }
}