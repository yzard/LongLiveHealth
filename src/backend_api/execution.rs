//! Service-owned admission for Tokio's blocking executor; no private thread pool.
use crate::error::AppError;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
pub struct CpuExecutor {
    slots: Arc<Semaphore>,
    capacity: u32,
    waiting: Arc<Semaphore>,
}
impl CpuExecutor {
    pub fn new(capacity: u32) -> Result<Self, AppError> {
        if capacity == 0 {
            return Err(AppError::Invalid("CPU capacity must be positive"));
        }
        Ok(Self {
            slots: Arc::new(Semaphore::new(capacity as usize)),
            capacity,
            waiting: Arc::new(Semaphore::new(4)),
        })
    }
    pub async fn run<F, T>(&self, work: F) -> Result<T, AppError>
    where
        F: FnOnce() -> Result<T, AppError> + Send + 'static,
        T: Send + 'static,
    {
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::RateLimited)?;
        Self::execute(permit, work).await
    }
    /// Bounded waiting for interactive aggregates; each queued caller retains at most
    /// one bounded query chunk. Upload admission remains fail-fast.
    pub async fn run_interactive<F, T>(&self, work: F) -> Result<T, AppError>
    where
        F: FnOnce() -> Result<T, AppError> + Send + 'static,
        T: Send + 'static,
    {
        let waiting = self
            .waiting
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::RateLimited)?;
        let permit = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.slots.clone().acquire_owned(),
        )
        .await
        .map_err(|_| AppError::RateLimited)?
        .map_err(|_| AppError::Internal)?;
        drop(waiting);
        Self::execute(permit, work).await
    }
    /// Durable service-owned workers wait until admitted or shutdown.
    pub(crate) async fn run_background<F, T>(&self, work: F) -> Result<T, AppError>
    where
        F: FnOnce() -> Result<T, AppError> + Send + 'static,
        T: Send + 'static,
    {
        let permit = self
            .slots
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AppError::Internal)?;
        Self::execute(permit, work).await
    }
    async fn execute<F, T>(permit: OwnedSemaphorePermit, work: F) -> Result<T, AppError>
    where
        F: FnOnce() -> Result<T, AppError> + Send + 'static,
        T: Send + 'static,
    {
        // Running closures retain admission after caller cancellation. Shutdown drains them.
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            work()
        })
        .await
        .map_err(|_| AppError::Internal)?
    }
    pub async fn shutdown(&self) {
        if let Ok(permits) = self.slots.clone().acquire_many_owned(self.capacity).await {
            self.slots.close();
            drop(permits);
        }
    }
}
