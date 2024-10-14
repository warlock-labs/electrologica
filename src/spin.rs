use std::hint::spin_loop;
use std::thread;
use std::time::{Duration, Instant};

// Constants for fine-tuning the spinning behavior
const SPIN_TRIES: u32 = 5;
const YIELD_TRIES: u32 = 10;
const PARK_TRIES: u32 = 15;
const INITIAL_BACKOFF_NANOS: u64 = 10;
const MAX_BACKOFF: Duration = Duration::from_micros(100);
const BACKOFF_FACTOR: u64 = 2;
pub const SPIN_TIMEOUT: Duration = Duration::from_micros(100000);

/// Executes a spinning strategy to repeatedly attempt an operation.
///
/// This function will try the given operation multiple times, using an escalating
/// strategy of spinning, yielding, and parking to reduce contention and CPU usage.
///
/// # Arguments
///
/// * `op` - A closure that returns `Some(T)` if successful, or `None` if the operation should be retried.
///
/// # Returns
///
/// Returns `Some(T)` if the operation succeeds within the timeout period, or `None` if it times out.
pub fn spin_try<T, F>(op: F) -> Option<T>
where
    F: Fn() -> Option<T>,
{
    let start = Instant::now();
    let mut spins = 0;
    let mut yields = 0;
    let mut parks = 0;

    while start.elapsed() < SPIN_TIMEOUT {
        if let Some(result) = op() {
            return Some(result);
        }

        if spins < SPIN_TRIES {
            spins += 1;
            spin_loop();
        } else if yields < YIELD_TRIES {
            yields += 1;
            thread::yield_now();
        } else if parks < PARK_TRIES {
            parks += 1;
            let backoff_nanos = INITIAL_BACKOFF_NANOS * BACKOFF_FACTOR.pow(parks);
            let backoff = Duration::from_nanos(backoff_nanos);
            thread::park_timeout(backoff.min(MAX_BACKOFF));
        } else {
            thread::park_timeout(MAX_BACKOFF);
        }
    }

    None
}
