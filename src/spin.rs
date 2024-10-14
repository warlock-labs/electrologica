//! Spin-based retry mechanism with configurable behavior and early termination.
//!
//! This module provides a configurable spin-wait implementation that can be used
//! for various concurrent programming scenarios where busy-waiting or
//! exponential backoff is needed. It includes an early termination feature
//! that breaks the spin and returns an error if no result is obtained by
//! the time the maximum backoff has elapsed or the timeout is reached.

use std::error::Error;
use std::fmt;
use std::hint::spin_loop;
use std::thread;
use std::time::{Duration, Instant};

// Constants for default spinning behavior
/// Default number of spin attempts before yielding
pub const SPIN_TRIES: u32 = 5;
/// Default number of yield attempts before parking
pub const YIELD_TRIES: u32 = 10;
/// Default number of park attempts with increasing duration
pub const PARK_TRIES: u32 = 15;
/// Default initial backoff duration in nanoseconds
pub const INITIAL_BACKOFF_NANOS: u64 = 10;
/// Default maximum backoff duration
pub const MAX_BACKOFF: Duration = Duration::from_micros(100);
/// Default factor by which to increase backoff duration
pub const BACKOFF_FACTOR: u64 = 2;
/// Default total timeout for the spinning operation
pub const SPIN_TIMEOUT: Duration = Duration::from_millis(100);

/// Configuration for the spinning behavior.
///
/// This struct allows fine-tuning of the spin-wait algorithm's parameters.
#[derive(Clone, Debug)]
pub struct SpinConfig {
    /// Number of times to spin before yielding
    pub spin_tries: u32,
    /// Number of times to yield before parking
    pub yield_tries: u32,
    /// Number of times to park with increasing duration before using max backoff
    pub park_tries: u32,
    /// Initial backoff duration in nanoseconds
    pub initial_backoff_nanos: u64,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Factor by which to increase backoff duration
    pub backoff_factor: u64,
    /// Total timeout for the spinning operation
    pub spin_timeout: Duration,
}

impl Default for SpinConfig {
    /// Creates a new `SpinConfig` with default values.
    ///
    /// This uses the module-level constants to set the default configuration.
    fn default() -> Self {
        SpinConfig {
            spin_tries: SPIN_TRIES,
            yield_tries: YIELD_TRIES,
            park_tries: PARK_TRIES,
            initial_backoff_nanos: INITIAL_BACKOFF_NANOS,
            max_backoff: MAX_BACKOFF,
            backoff_factor: BACKOFF_FACTOR,
            spin_timeout: SPIN_TIMEOUT,
        }
    }
}

/// Error type for spin-wait operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SpinError {
    /// The operation timed out after reaching the maximum backoff.
    MaxBackoffReached,
    /// The operation timed out after reaching the total spin timeout.
    Timeout,
}

impl fmt::Display for SpinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpinError::MaxBackoffReached => write!(f, "Maximum backoff reached without result"),
            SpinError::Timeout => write!(f, "Operation timed out"),
        }
    }
}

impl Error for SpinError {}

/// Executes a spinning strategy to repeatedly attempt an operation with configurable parameters.
///
/// This function will try the given operation multiple times, using an escalating
/// strategy of spinning, yielding, and parking to reduce contention and CPU usage.
/// It will break the spin and return an error if no result is obtained by the time
/// the maximum backoff has elapsed or the timeout is reached.
///
/// # Arguments
///
/// * `op` - A closure that returns `Some(T)` if successful, or `None` if the operation should be retried.
/// * `config` - A `SpinConfig` struct containing the configuration parameters for the spinning behavior.
///
/// # Returns
///
/// Returns `Ok(T)` if the operation succeeds, or an `Err(SpinError)` if it fails to produce a result
/// within the specified constraints.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use std::thread;
/// use std::time::Duration;
/// use electrologica::spin::{spin_try, SpinConfig, SpinError};
///
/// let flag = Arc::new(AtomicBool::new(false));
/// let flag_clone = Arc::clone(&flag);
///
/// // Spawn a thread that will set the flag after a short delay
/// thread::spawn(move || {
///     thread::sleep(Duration::from_millis(10));
///     flag_clone.store(true, Ordering::SeqCst);
/// });
///
/// // Use spin_try to wait for the flag to be set
/// let result = spin_try(
///     || {
///         if flag.load(Ordering::SeqCst) {
///             Some(true)
///         } else {
///             None
///         }
///     },
///     SpinConfig {
///         spin_timeout: Duration::from_millis(100),
///         ..SpinConfig::default()
///     }
/// );
///
/// assert!(result.is_ok(), "Expected Ok, got {:?}", result);
/// assert_eq!(result.unwrap(), true);
/// ```
pub fn spin_try<T, F>(op: F, config: SpinConfig) -> Result<T, SpinError>
where
    F: Fn() -> Option<T>,
{
    let start = Instant::now();
    let mut spins = 0;
    let mut yields = 0;
    let mut parks = 0;

    while start.elapsed() < config.spin_timeout {
        // Attempt the operation
        if let Some(result) = op() {
            return Ok(result);
        }

        if spins < config.spin_tries {
            spins += 1;
            spin_loop();
        } else if yields < config.yield_tries {
            yields += 1;
            thread::yield_now();
        } else if parks < config.park_tries {
            parks += 1;
            let backoff_nanos = config.initial_backoff_nanos * config.backoff_factor.pow(parks);
            let backoff = Duration::from_nanos(backoff_nanos).min(config.max_backoff);

            // Use a shorter park duration and check the condition more frequently
            let park_start = Instant::now();
            while park_start.elapsed() < backoff {
                thread::park_timeout(Duration::from_millis(1));
                if let Some(result) = op() {
                    return Ok(result);
                }
                if start.elapsed() >= config.spin_timeout {
                    return Err(SpinError::Timeout);
                }
            }
        } else {
            return Err(SpinError::MaxBackoffReached);
        }

        if start.elapsed() >= config.spin_timeout {
            return Err(SpinError::Timeout);
        }
    }

    Err(SpinError::Timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_spin_try_success() {
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&flag);

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1));
            flag_clone.store(true, Ordering::SeqCst);
        });

        let result = spin_try(
            || {
                if flag.load(Ordering::SeqCst) {
                    Some(true)
                } else {
                    None
                }
            },
            SpinConfig {
                spin_timeout: Duration::from_millis(100),
                ..Default::default()
            },
        );

        handle.join().unwrap();

        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert!(result.unwrap());
    }

    #[test]
    fn test_spin_try_max_backoff_reached() {
        let result: Result<(), SpinError> = spin_try(
            || None,
            SpinConfig {
                spin_tries: 1,
                yield_tries: 1,
                park_tries: 1,
                spin_timeout: Duration::from_secs(1),
                ..Default::default()
            },
        );

        assert!(
            matches!(
                result,
                Err(SpinError::MaxBackoffReached) | Err(SpinError::Timeout)
            ),
            "Expected MaxBackoffReached or Timeout, got {:?}",
            result
        );
    }

    #[test]
    fn test_spin_try_timeout() {
        let result: Result<(), SpinError> = spin_try(
            || None,
            SpinConfig {
                spin_timeout: Duration::from_millis(50),
                spin_tries: 1000000, // Set this high to ensure we hit the timeout
                yield_tries: 0,
                park_tries: 0,
                ..Default::default()
            },
        );

        assert!(
            matches!(result, Err(SpinError::Timeout)),
            "Expected Timeout, got {:?}",
            result
        );
    }
}
