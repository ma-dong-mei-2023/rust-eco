use std::sync::Arc;
use tokio::sync::{Mutex as TokioMutex, RwLock, Semaphore, Barrier};
use tokio::sync::{Notify, oneshot, broadcast, mpsc};

pub struct Mutex<T> {
    inner: Arc<TokioMutex<T>>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(TokioMutex::new(value)),
        }
    }

    pub async fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard {
            guard: self.inner.lock().await,
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        self.inner.try_lock().ok().map(move |guard| MutexGuard { guard })
    }
}

impl<T> Clone for Mutex<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct MutexGuard<'a, T> {
    guard: tokio::sync::MutexGuard<'a, T>,
}

impl<'a, T> std::ops::Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> std::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

pub struct ReadWriteLock<T> {
    inner: Arc<RwLock<T>>,
}

impl<T> ReadWriteLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(value)),
        }
    }

    pub async fn read(&self) -> ReadGuard<'_, T> {
        ReadGuard {
            guard: self.inner.read().await,
        }
    }

    pub async fn write(&self) -> WriteGuard<'_, T> {
        WriteGuard {
            guard: self.inner.write().await,
        }
    }

    pub fn try_read(&self) -> Option<ReadGuard<'_, T>> {
        self.inner.try_read().ok().map(move |guard| ReadGuard { guard })
    }

    pub fn try_write(&self) -> Option<WriteGuard<'_, T>> {
        self.inner.try_write().ok().map(move |guard| WriteGuard { guard })
    }
}

impl<T> Clone for ReadWriteLock<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct ReadGuard<'a, T> {
    guard: tokio::sync::RwLockReadGuard<'a, T>,
}

impl<'a, T> std::ops::Deref for ReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

pub struct WriteGuard<'a, T> {
    guard: tokio::sync::RwLockWriteGuard<'a, T>,
}

impl<'a, T> std::ops::Deref for WriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> std::ops::DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

pub struct WaitGroup {
    semaphore: Arc<Semaphore>,
    count: Arc<TokioMutex<usize>>,
}

impl WaitGroup {
    pub fn new() -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(0)),
            count: Arc::new(TokioMutex::new(0)),
        }
    }

    pub async fn add(&self, n: usize) {
        let mut count = self.count.lock().await;
        *count += n;
    }

    pub async fn done(&self) {
        let mut count = self.count.lock().await;
        if *count > 0 {
            *count -= 1;
            if *count == 0 {
                self.semaphore.add_permits(1);
            }
        }
    }

    pub async fn wait(&self) {
        let count = {
            let count = self.count.lock().await;
            *count
        };
        
        if count == 0 {
            return;
        }

        let _permit = self.semaphore.acquire().await.unwrap();
    }
}

impl Clone for WaitGroup {
    fn clone(&self) -> Self {
        Self {
            semaphore: self.semaphore.clone(),
            count: self.count.clone(),
        }
    }
}

impl Default for WaitGroup {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Condition {
    notify: Arc<Notify>,
    mutex: Arc<TokioMutex<()>>,
}

impl Condition {
    pub fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
            mutex: Arc::new(TokioMutex::new(())),
        }
    }

    pub async fn wait(&self) {
        let _guard = self.mutex.lock().await;
        self.notify.notified().await;
    }

    pub fn notify_one(&self) {
        self.notify.notify_one();
    }

    pub fn notify_all(&self) {
        self.notify.notify_waiters();
    }
}

impl Clone for Condition {
    fn clone(&self) -> Self {
        Self {
            notify: self.notify.clone(),
            mutex: self.mutex.clone(),
        }
    }
}

impl Default for Condition {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Once {
    done: Arc<TokioMutex<bool>>,
    notify: Arc<Notify>,
}

impl Once {
    pub fn new() -> Self {
        Self {
            done: Arc::new(TokioMutex::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn call_once<F, Fut>(&self, f: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let mut done = self.done.lock().await;
        if !*done {
            f().await;
            *done = true;
            self.notify.notify_waiters();
        } else {
            drop(done);
            self.notify.notified().await;
        }
    }

    pub async fn is_done(&self) -> bool {
        *self.done.lock().await
    }
}

impl Clone for Once {
    fn clone(&self) -> Self {
        Self {
            done: self.done.clone(),
            notify: self.notify.clone(),
        }
    }
}

impl Default for Once {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_mutex() {
        let mutex = Mutex::new(0);
        let mutex_clone = mutex.clone();

        let handle = tokio::spawn(async move {
            let mut guard = mutex_clone.lock().await;
            *guard += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
            *guard += 1;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        
        {
            let guard = mutex.lock().await;
            assert!(*guard >= 1);
        }

        handle.await.unwrap();
        
        let guard = mutex.lock().await;
        assert_eq!(*guard, 2);
    }

    #[tokio::test]
    async fn test_rwlock() {
        let lock = ReadWriteLock::new(0);
        let lock_clone = lock.clone();

        // Multiple readers
        let read1 = tokio::spawn({
            let lock = lock.clone();
            async move {
                let guard = lock.read().await;
                tokio::time::sleep(Duration::from_millis(100)).await;
                *guard
            }
        });

        let read2 = tokio::spawn({
            let lock = lock.clone();
            async move {
                let guard = lock.read().await;
                tokio::time::sleep(Duration::from_millis(100)).await;
                *guard
            }
        });

        let (val1, val2) = tokio::join!(read1, read2);
        assert_eq!(val1.unwrap(), 0);
        assert_eq!(val2.unwrap(), 0);

        // Single writer
        {
            let mut guard = lock_clone.write().await;
            *guard = 42;
        }

        let guard = lock.read().await;
        assert_eq!(*guard, 42);
    }

    #[tokio::test]
    async fn test_wait_group() {
        let wg = WaitGroup::new();
        let wg_clone = wg.clone();

        wg.add(2).await;

        let handle1 = tokio::spawn({
            let wg = wg.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                wg.done().await;
            }
        });

        let handle2 = tokio::spawn({
            let wg = wg.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                wg.done().await;
            }
        });

        wg_clone.wait().await;

        handle1.await.unwrap();
        handle2.await.unwrap();
    }

    #[tokio::test]
    async fn test_condition() {
        let cond = Condition::new();
        let cond_clone = cond.clone();

        let waiter = tokio::spawn(async move {
            cond_clone.wait().await;
            "notified"
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        cond.notify_one();

        let result = waiter.await.unwrap();
        assert_eq!(result, "notified");
    }

    #[tokio::test]
    async fn test_once() {
        let once = Once::new();
        let mut counter = Arc::new(TokioMutex::new(0));

        let handles: Vec<_> = (0..10).map(|_| {
            let once = once.clone();
            let counter = counter.clone();
            tokio::spawn(async move {
                once.call_once(|| async {
                    let mut c = counter.lock().await;
                    *c += 1;
                }).await;
            })
        }).collect();

        for handle in handles {
            handle.await.unwrap();
        }

        let final_count = *counter.lock().await;
        assert_eq!(final_count, 1);
        assert!(once.is_done().await);
    }
}