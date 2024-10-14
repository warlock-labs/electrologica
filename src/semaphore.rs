use crate::spin::{spin_try, SpinConfig, SpinError};
use std::sync::atomic::{AtomicU64, Ordering};

/// A high-performance, atomic semaphore optimized for extremely low latency.
///
/// This semaphore is designed for scenarios where acquisition times in the range of
/// 100 nanoseconds are desirable, and 500 microseconds is considered too slow.
///
/// # Examples
///
/// ```
/// use electrologica::AtomicSemaphore;
///
/// let sem = AtomicSemaphore::new(5);
/// assert!(sem.try_acquire());
/// assert_eq!(sem.available_permits(), 4);
/// sem.release();
/// assert_eq!(sem.available_permits(), 5);
/// ```
#[derive(Debug)]
pub struct AtomicSemaphore {
    /// The current count of available permits
    permits: AtomicU64,
    /// The maximum number of permits this semaphore can issue
    max_permits: u64,
}

impl AtomicSemaphore {
    /// Creates a new `AtomicSemaphore` with the specified number of permits.
    ///
    /// # Arguments
    ///
    /// * `count` - The initial number of permits available.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicSemaphore;
    ///
    /// let sem = AtomicSemaphore::new(5);
    /// ```
    pub fn new(count: u64) -> Self {
        AtomicSemaphore {
            permits: AtomicU64::new(count),
            max_permits: count,
        }
    }

    /// Attempts to acquire a permit from the semaphore, retrying with a spin-wait strategy if no permits are immediately available.
    ///
    /// This method will keep trying to acquire a permit using `try_acquire` until it succeeds or the spin-wait strategy times out.
    ///
    /// # Returns
    ///
    /// * `true` if a permit was successfully acquired.
    /// * `false` if no permit could be acquired after all retry attempts.
    ///
    /// # Example
    ///
    /// ```
    /// use electrologica::AtomicSemaphore;
    ///
    /// let sem = AtomicSemaphore::new(1);
    /// assert!(sem.acquire());
    /// assert!(!sem.acquire()); // No more permits available
    /// sem.release();
    /// assert!(sem.acquire()); // Now we can acquire again
    /// ```
    pub fn acquire(&self) -> bool {
        match spin_try(
            || if self.try_acquire() { Some(()) } else { None },
            SpinConfig::default(),
        ) {
            Ok(()) => true,
            Err(SpinError::MaxBackoffReached) | Err(SpinError::Timeout) => false,
        }
    }

    /// Attempts to acquire a permit from the semaphore using a custom SpinConfig.
    ///
    /// This method behaves like `acquire`, but uses the provided SpinConfig
    /// instead of the default one.
    ///
    /// # Arguments
    ///
    /// * `config` - A custom `SpinConfig` to use for this acquire attempt.
    ///
    /// # Returns
    ///
    /// * `true` if a permit was successfully acquired.
    /// * `false` if no permit could be acquired after all retry attempts.
    pub fn acquire_with_config(&self, config: SpinConfig) -> bool {
        match spin_try(|| if self.try_acquire() { Some(()) } else { None }, config) {
            Ok(()) => true,
            Err(SpinError::MaxBackoffReached) | Err(SpinError::Timeout) => false,
        }
    }

    /// Attempts to acquire a permit without blocking.
    ///
    /// This method will immediately return `true` if a permit is available,
    /// or `false` if no permits are currently available.
    ///
    /// # Returns
    ///
    /// * `true` if a permit was acquired
    /// * `false` if no permits were available
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicSemaphore;
    ///
    /// let sem = AtomicSemaphore::new(1);
    /// assert!(sem.try_acquire());
    /// assert!(!sem.try_acquire()); // No more permits available
    /// ```
    pub fn try_acquire(&self) -> bool {
        let mut current = self.permits.load(Ordering::Acquire);
        while current > 0 {
            match self.permits.compare_exchange(
                current,
                current - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
        false
    }

    /// Releases a permit back to the semaphore.
    ///
    /// This method increases the number of available permits by one, but will not
    /// increase the count beyond the maximum number of permits.
    ///
    /// # Returns
    ///
    /// * `true` if the permit was successfully released
    /// * `false` if the permit could not be released (already at max permits)
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicSemaphore;
    ///
    /// let sem = AtomicSemaphore::new(1);
    /// sem.acquire();
    /// assert!(sem.release());
    /// assert!(!sem.release()); // Can't release more than the max permits
    /// ```
    pub fn release(&self) -> bool {
        let mut current = self.permits.load(Ordering::Acquire);
        while current < self.max_permits {
            match self.permits.compare_exchange(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
        false
    }

    /// Returns the current number of available permits.
    ///
    /// This method may be inaccurate in the presence of concurrent operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicSemaphore;
    ///
    /// let sem = AtomicSemaphore::new(5);
    /// assert_eq!(sem.available_permits(), 5);
    /// sem.acquire();
    /// assert_eq!(sem.available_permits(), 4);
    /// ```
    pub fn available_permits(&self) -> u64 {
        self.permits.load(Ordering::Acquire)
    }
}

/// RAII guard for automatic release of a semaphore permit.
pub struct SemaphoreGuard<'a> {
    semaphore: &'a AtomicSemaphore,
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

impl AtomicSemaphore {
    /// Acquires a permit and returns a RAII guard.
    ///
    /// The permit is automatically released when the guard is dropped.
    ///
    /// # Returns
    ///
    /// * `Some(SemaphoreGuard)` if a permit was acquired
    /// * `None` if the acquisition timed out
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicSemaphore;
    ///
    /// let sem = AtomicSemaphore::new(1);
    /// {
    ///     let guard = sem.acquire_guard();
    ///     assert!(guard.is_some());
    ///     // The semaphore is held here
    /// } // The semaphore is automatically released here
    /// assert!(sem.try_acquire());
    /// ```
    pub fn acquire_guard(&self) -> Option<SemaphoreGuard> {
        if self.acquire() {
            Some(SemaphoreGuard { semaphore: self })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use std::thread;
    use std::time::Instant;

    use once_cell::sync::Lazy;
    use rayon::iter::{IntoParallelIterator, ParallelIterator};
    use tracing::info;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{fmt, EnvFilter};

    use super::*;

    pub static TRACING: Lazy<()> = Lazy::new(|| {
        let fmt_layer = fmt::layer()
            .pretty()
            .with_line_number(true)
            .with_thread_names(true)
            .with_thread_ids(true)
            .with_target(false);

        let filter_layer =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();
    });

    #[test]
    fn test_new_semaphore() {
        let sem = AtomicSemaphore::new(5);
        assert_eq!(sem.available_permits(), 5);
    }

    #[test]
    fn test_acquire_and_release() {
        let sem = AtomicSemaphore::new(2);
        assert!(sem.acquire());
        assert!(sem.acquire());
        assert!(!sem.try_acquire());
        assert!(sem.release());
        assert!(sem.acquire());
    }

    #[test]
    fn test_try_acquire() {
        let sem = AtomicSemaphore::new(1);
        assert!(sem.try_acquire());
        assert!(!sem.try_acquire());
        sem.release();
        assert!(sem.try_acquire());
    }

    #[test]
    fn test_max_count() {
        let sem = AtomicSemaphore::new(2);
        assert!(sem.acquire());
        assert!(sem.acquire());
        assert!(!sem.acquire());
        assert!(sem.release());
        assert!(sem.release());
        assert!(!sem.release()); // This should not increase the count beyond 2
        assert!(sem.acquire());
        assert!(sem.acquire());
        assert!(!sem.acquire());
        assert_eq!(sem.available_permits(), 0);
    }

    #[test]
    fn test_guard() {
        let sem = AtomicSemaphore::new(1);
        {
            let guard = sem.acquire_guard();
            assert!(guard.is_some());
            assert!(!sem.try_acquire());
        }
        assert!(sem.try_acquire());
    }

    #[test]
    fn test_semaphore_with_rayon() {
        Lazy::force(&TRACING);
        let parallelism = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1);
        let total_operations = 1_000_000;

        let sem = Arc::new(AtomicSemaphore::new(parallelism as u64));
        let counter = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));
        let operations_completed = Arc::new(AtomicUsize::new(0));
        let operations_attempted = Arc::new(AtomicUsize::new(0));

        let start = Instant::now();

        (0..total_operations).into_par_iter().for_each(|_| {
            operations_attempted.fetch_add(1, Ordering::Relaxed);
            match sem.acquire_guard() {
                Some(_guard) => {
                    let current = counter.fetch_add(1, Ordering::Relaxed);
                    max_concurrent.fetch_max(current + 1, Ordering::Relaxed);

                    // Simulate some work
                    std::hint::spin_loop();

                    counter.fetch_sub(1, Ordering::Relaxed);
                    operations_completed.fetch_add(1, Ordering::Relaxed);
                }
                None => {
                    // This branch should never be reached in this test
                    panic!("Failed to acquire semaphore guard");
                }
            }
        });

        let duration = start.elapsed();
        let total_completed = operations_completed.load(Ordering::Relaxed);
        let total_attempted = operations_attempted.load(Ordering::Relaxed);

        assert_eq!(
            counter.load(Ordering::SeqCst),
            0,
            "All operations should have completed"
        );
        assert!(
            max_concurrent.load(Ordering::SeqCst) <= parallelism,
            "Max concurrent operations exceeded parallelism * 2"
        );
        assert_eq!(
            sem.available_permits(),
            parallelism as u64,
            "All permits should have been returned"
        );
        assert_eq!(
            total_completed, total_operations,
            "All operations should have completed"
        );
        assert_eq!(
            total_attempted, total_operations,
            "All operations should have been attempted"
        );

        info!("Rayon-based high concurrency test results:");
        info!("Total operations attempted: {}", total_attempted);
        info!("Total operations completed: {}", total_completed);
        info!("Parallelism (max concurrent operations): {}", parallelism);
        info!(
            "Actual max concurrent operations: {}",
            max_concurrent.load(Ordering::SeqCst)
        );
        info!("Total duration: {:?}", duration);
        info!(
            "Operations per second: {:.2}",
            total_completed as f64 / duration.as_secs_f64()
        );
    }

    #[test]
    fn test_concurrent_acquire_release() {
        let sem = Arc::new(AtomicSemaphore::new(5));
        let threads: Vec<_> = (0..10)
            .map(|_| {
                let sem = Arc::clone(&sem);
                thread::spawn(move || {
                    for _ in 0..1000 {
                        if let Some(_guard) = sem.acquire_guard() {
                            thread::yield_now(); // Simulate some work
                        }
                    }
                })
            })
            .collect();

        for thread in threads {
            thread.join().unwrap();
        }

        assert_eq!(sem.available_permits(), 5);
    }

    #[test]
    fn test_timeout() {
        use crate::spin::SpinConfig;
        use std::time::Duration;

        let sem = AtomicSemaphore::new(1);
        assert!(sem.acquire(), "First acquire should succeed");

        // Define a specific timeout for this test
        let test_timeout = Duration::from_millis(100);
        let config = SpinConfig {
            spin_timeout: test_timeout,
            ..SpinConfig::default()
        };

        let start = Instant::now();

        // Use a custom SpinConfig for this acquire call
        assert!(
            !sem.acquire_with_config(config),
            "Second acquire should fail due to timeout"
        );

        let elapsed = start.elapsed();

        // Just ensure that some time has passed
        assert!(
            elapsed > Duration::from_millis(1),
            "Operation returned too quickly"
        );

        // Log the actual elapsed time for informational purposes
        println!("Acquire operation with timeout took {:?}", elapsed);

        // Ensure we can acquire again after releasing
        sem.release();
        assert!(sem.acquire(), "Should be able to acquire after release");
    }
}
