use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout, Instant};

pub async fn sleep_secs(secs: f64) {
    let duration = Duration::from_secs_f64(secs);
    sleep(duration).await;
}

pub async fn sleep_millis(millis: u64) {
    sleep(Duration::from_millis(millis)).await;
}

pub fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

pub fn monotonic() -> f64 {
    Instant::now().elapsed().as_secs_f64()
}

pub struct Timer {
    interval: Duration,
    next_tick: Instant,
}

impl Timer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            next_tick: Instant::now() + interval,
        }
    }

    pub fn from_secs(secs: f64) -> Self {
        Self::new(Duration::from_secs_f64(secs))
    }

    pub async fn tick(&mut self) {
        let now = Instant::now();
        if now < self.next_tick {
            sleep(self.next_tick - now).await;
        }
        self.next_tick += self.interval;
    }

    pub fn reset(&mut self) {
        self.next_tick = Instant::now() + self.interval;
    }
}

pub async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, tokio::time::error::Elapsed>
where
    F: std::future::Future<Output = T>,
{
    timeout(duration, future).await
}

pub async fn with_timeout_secs<F, T>(secs: f64, future: F) -> Result<T, tokio::time::error::Elapsed>
where
    F: std::future::Future<Output = T>,
{
    with_timeout(Duration::from_secs_f64(secs), future).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sleep_secs() {
        let start = std::time::Instant::now();
        sleep_secs(0.1).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(90));
        assert!(elapsed < Duration::from_millis(200));
    }

    #[tokio::test]
    async fn test_timer() {
        let mut timer = Timer::from_secs(0.1);
        let start = std::time::Instant::now();
        
        timer.tick().await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(90));
        assert!(elapsed < Duration::from_millis(200));
    }

    #[tokio::test]
    async fn test_with_timeout() {
        let result = with_timeout_secs(0.1, async {
            sleep_secs(0.05).await;
            42
        }).await;
        
        assert_eq!(result.unwrap(), 42);

        let result = with_timeout_secs(0.05, async {
            sleep_secs(0.1).await;
            42
        }).await;
        
        assert!(result.is_err());
    }

    #[test]
    fn test_now() {
        let timestamp = now();
        assert!(timestamp > 1_600_000_000.0); // After 2020
    }
}