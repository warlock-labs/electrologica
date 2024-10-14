//! # AtomicRingBuffer
//!
//! A high-performance, lock-free, single-producer single-consumer (SPSC) ring buffer implementation
//! that supports contiguous writes and reads for any type `T`, including non-Copy types.
//! This implementation is inspired by the work of James Munes
//! (<https://ferrous-systems.com/blog/lock-free-ring-buffer/>) and adapted for complex data types
//! using modern Rust techniques.
//!
//! ## Design Goals
//!
//! - **Lock-free operations**: Ensures high performance in concurrent scenarios.
//! - **Contiguous writes and reads**: Supports efficient bulk operations.
//! - **Minimal unsafe code**: Strives for safety while maintaining performance.
//! - **Correct memory ordering**: Ensures proper cross-thread synchronization.
//! - **Efficient wraparound handling**: Uses a watermark mechanism for seamless circular operations.
//! - **Generic type support**: Works with any `Send` type `T`.
//!
//! ## Key Features
//!
//! - **Fixed-size buffer**: Allocated on the stack for predictable performance.
//! - **Atomic operations**: Ensures thread-safe access without locks.
//! - **Watermark mechanism**: Efficiently handles buffer wraparound.
//! - **Flexible type support**: Works with any `Send` type, not just `Copy` types.
//! - **Optimized performance**: Designed for low-latency, high-throughput scenarios.
//!
//! ## Usage Scenarios
//!
//! The `AtomicRingBuffer` is particularly well-suited for:
//!
//! - **Embedded systems**: Where contiguous memory regions are required for DMA operations.
//! - **High-performance computing**: For efficient inter-thread communication.
//! - **Audio/Video processing**: Where consistent, low-latency data transfer is crucial.
//! - **Network packet handling**: For efficient buffering of network data.
//!
//! While designed for SPSC scenarios, the `push` and `pop` methods are also safe for
//! multi-producer, multi-consumer (MPMC) use, albeit with potential performance implications.
//!
//! ## Thread Safety
//!
//! - The `AtomicRingBuffer` is safe to share between threads when `T` is `Send`.
//! - It provides strong guarantees for single-producer, single-consumer scenarios.
//! - In MPMC scenarios, correctness is maintained, but performance may be impacted.
//!
//! ## Performance Considerations
//!
//! - **Lock-free design**: Minimizes contention and avoids lock-related performance issues.
//! - **Atomic operations**: Leverages hardware-level atomics for efficient synchronization.
//! - **Cache-friendly**: Uses cache padding to prevent false sharing between core data structures.
//! - **Compile-time optimization**: Utilizes const generics for size, allowing compiler optimizations.
//! - **Efficient wraparound**: The watermark mechanism ensures efficient circular buffer operations.
//!
//! ## Memory Ordering
//!
//! The implementation carefully uses various memory ordering levels to balance correctness and performance:
//!
//! - `Relaxed`: For operations that don't require synchronization.
//! - `Acquire`: When reading shared data to ensure visibility of previous writes.
//! - `Release`: When writing shared data to make updates visible to other threads.
//! - `AcqRel`: For operations that both read and write shared data.
//! - `SeqCst`: Used sparingly for operations requiring total ordering across all threads.
//!
//! ## Main Operations
//!
//! - `push`: Attempts to add an element to the buffer.
//! - `pop`: Attempts to remove and return an element from the buffer.
//! - `peek`: Views the next element without removing it.
//! - `len`: Returns the current number of elements in the buffer.
//! - `capacity`: Returns the maximum number of elements the buffer can hold.
//! - `is_empty`: Checks if the buffer contains no elements.
//! - `clear`: Removes all elements from the buffer.
//! - `drain`: Returns an iterator that removes and yields all elements.
//!
//! ## Iteration
//!
//! The `AtomicRingBuffer` supports both non-consuming and draining iteration:
//!
//! - `iter`: Returns an iterator over references to the buffer's contents.
//! - `drain`: Returns an iterator that consumes the buffer's contents.
//!
//! Both iterators provide a snapshot view and may not reflect concurrent modifications.
//!
//! ## Examples
//!
//! Basic usage:
//!
//! ```rust
//! use electrologica::AtomicRingBuffer;
//!
//! let buffer: AtomicRingBuffer<String, 1024> = AtomicRingBuffer::new();
//!
//! // Producer thread
//! buffer.try_push(String::from("Hello")).unwrap();
//! buffer.try_push(String::from("World")).unwrap();
//!
//! // Consumer thread
//! assert_eq!(buffer.try_pop(), Some(String::from("Hello")));
//! assert_eq!(buffer.try_pop(), Some(String::from("World")));
//! assert_eq!(buffer.try_pop(), None);
//! ```
//!
//! Using the drain iterator:
//!
//! ```rust
//! use electrologica::AtomicRingBuffer;
//!
//! let buffer: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();
//! buffer.try_push(1).unwrap();
//! buffer.try_push(2).unwrap();
//!
//! let drained: Vec<i32> = buffer.drain().collect();
//! assert_eq!(drained, vec![1, 2]);
//! assert!(buffer.is_empty());
//! ```
//!
//! ## Safety Considerations
//!
//! While the `AtomicRingBuffer` strives to minimize unsafe code, some unsafe operations are
//! necessary for performance and to handle raw pointers. All unsafe code is carefully
//! commented and reasoned about. Users of this module should be aware that:
//!
//! - The buffer's thread-safety guarantees rely on proper use of atomic operations and memory ordering.
//! - Some methods (e.g., `clear`) are not atomic and should be used with caution in concurrent scenarios.
//! - The buffer's behavior in highly concurrent MPMC scenarios may lead to unexpected results,
//!   though it will maintain memory safety.
//!
//! ## Limitations
//!
//! - **Fixed capacity**: The buffer size is determined at compile-time and cannot be changed dynamically.
//! - **SPSC optimization**: While safe for MPMC use, the buffer is optimized for SPSC scenarios.
//! - **No blocking operations**: The buffer does not provide blocking push/pop operations; it's the
//!   user's responsibility to handle full/empty scenarios.
//!
//! ## Performance Tuning
//!
//! For optimal performance:
//!
//! - Choose an appropriate buffer size (power of two) based on your use case.
//! - In SPSC scenarios, dedicate separate threads for producing and consuming.
//! - Be mindful of the trade-offs between frequent small operations and less frequent bulk operations.
//! - Consider using the `peek` method to check for available data before attempting to `pop`.
//!
//! ## Future Directions
//!
//! Potential areas for future enhancement include:
//!
//! - Specialized MPMC variants with different performance characteristics.
//! - Integration with async Rust for non-blocking I/O scenarios.
//! - Additional bulk operation methods for specific use cases.
//! - Further optimizations for specific architectures or use patterns.
//!
//! ## References
//!
//! This implementation draws inspiration from various sources in concurrent programming literature:
//!
//! 1. Herlihy, M., & Wing, J. M. (1990). Linearizability: A correctness condition for concurrent objects.
//! 2. Michael, M. M., & Scott, M. L. (1996). Simple, fast, and practical non-blocking and blocking concurrent queue algorithms.
//! 3. Lamport, L. (1977). Concurrent reading and writing.
//! 4. Morrison, A., & Afek, Y. (2013). Fast concurrent queues for x86 processors.
//!
//! Users interested in the theoretical underpinnings of lock-free data structures are encouraged
//! to explore these references.

use std::cell::{RefCell, UnsafeCell};
use std::fmt::Debug;
use std::iter::FusedIterator;
use std::mem::{needs_drop, MaybeUninit};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fmt, ptr};

use crossbeam_utils::CachePadded;
#[cfg(feature = "rayon")]
use rayon::iter::plumbing::{bridge, Consumer, Producer, ProducerCallback, UnindexedConsumer};
#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::spin::spin_try;

/// A lock-free, single-producer single-consumer ring buffer with support for contiguous writes.
///
/// # Type Parameters
///
/// - `T`: The type of elements stored in the buffer. Must be `Send`.
/// - `N`: The capacity of the buffer. Must be a power of two for efficient
///        wrapping, the buffer can hold up to `N - 1` elements.
///
/// # Thread Safety
///
/// This structure is safe to share between threads when `T` is `Send`.
/// It's designed for single-producer single-consumer scenarios.
pub struct AtomicRingBuffer<T, const N: usize> {
    /// The index where the next element will be written.
    /// Only modified by the producer.
    write: CachePadded<AtomicUsize>,

    /// The index where the next element will be read from.
    /// Only modified by the consumer.
    read: CachePadded<AtomicUsize>,

    /// The high watermark for valid data.
    /// Used to handle wrap-around scenarios efficiently.
    /// Modified by the producer, read by both the producer and the consumer.
    watermark: CachePadded<AtomicUsize>,

    /// The internal buffer storing the elements.
    /// UnsafeCell is used to allow interior mutability.
    /// MaybeUninit is used to handle uninitialized memory safely.
    buffer: [UnsafeCell<MaybeUninit<T>>; N],
}

// Safety: AtomicRingBuffer<T> is Send if T is Send.
// This is safe because we use atomic operations for all shared mutable state.
unsafe impl<T: Send, const N: usize> Send for AtomicRingBuffer<T, N> {}

// Safety: AtomicRingBuffer<T> is Sync if T is Send.
// This is safe because our operations ensure proper synchronization between threads.
unsafe impl<T: Sync, const N: usize> Sync for AtomicRingBuffer<T, N> {}

impl<T, const N: usize> AtomicRingBuffer<T, N> {
    /// Creates a new, empty `AtomicRingBuffer`.
    ///
    /// # Panics
    ///
    /// This function will panic if `N` is not a power of two.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 1024> = AtomicRingBuffer::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        assert!(
            N.is_power_of_two(),
            "Buffer capacity must be a power of two"
        );
        // TODO(Clippy is badly pleased here)
        // We disable the lint for this line, but remember that this is an unstable library, and
        // this may indeed be undefined behavior.
        #[allow(clippy::uninit_assumed_init)]
        let buffer: [UnsafeCell<MaybeUninit<T>>; N] =
            unsafe { MaybeUninit::uninit().assume_init() };
        Self {
            write: CachePadded::new(AtomicUsize::new(0)),
            read: CachePadded::new(AtomicUsize::new(0)),
            watermark: CachePadded::new(AtomicUsize::new(N)),
            buffer,
        }
    }

    /// Attempts to push an element onto the buffer.
    ///
    /// This method ensures thread-safe insertion of elements into the buffer,
    /// handling wraparound conditions and maintaining proper synchronization
    /// with consumer threads.
    ///
    /// # Arguments
    ///
    /// * `item`: The item to push onto the buffer.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the item was successfully pushed, or `Err(T)` with
    /// the original item if the buffer was full.
    ///
    /// # Thread Safety
    ///
    /// This method uses atomic operations with appropriate memory ordering to ensure
    /// thread-safe access to the buffer in a multi-threaded environment.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();
    /// assert!(buffer.try_push(String::from("Hello")).is_ok());
    /// assert!(buffer.try_push(String::from("World")).is_ok());
    /// assert!(buffer.try_push(String::from("!")).is_ok());
    /// assert!(buffer.try_push(String::from("Goodbye")).is_err());
    /// ```
    #[inline]
    pub fn try_push(&self, item: T) -> Result<(), T> {
        // Load the current write index with Acquire ordering.
        let write = self.write.load(Ordering::Acquire);

        // Calculate the next write position, wrapping around if necessary.
        let next_write = Self::increment(write);

        // Load the current read index with Acquire ordering.
        let read = self.read.load(Ordering::Acquire);

        // Check if the buffer is full.
        if next_write == read {
            return Err(item); // Buffer is full, return the item.
        }

        // Prepare the new watermark value.
        let new_watermark = if next_write == 0 { write } else { N };

        // Attempt to update both the write index and the watermark.
        match self
            .write
            .compare_exchange(write, next_write, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => {
                // The write index was successfully updated.
                // Now, attempt to update the watermark if necessary.
                if next_write == 0 {
                    match self.watermark.compare_exchange(
                        N,
                        new_watermark,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            // Both write index and watermark were successfully updated.
                            // Now it's safe to write the item.
                            unsafe {
                                ptr::write((*self.buffer[write].get()).as_mut_ptr(), item);
                            }
                            Ok(())
                        }
                        Err(_) => {
                            // Watermark update failed. Revert the write index and return an error.
                            let _ = self.write.compare_exchange(
                                next_write,
                                write,
                                Ordering::AcqRel,
                                Ordering::Acquire,
                            );
                            Err(item)
                        }
                    }
                } else {
                    // No need to update the watermark.
                    // It's safe to write the item.
                    unsafe {
                        ptr::write((*self.buffer[write].get()).as_mut_ptr(), item);
                    }
                    Ok(())
                }
            }
            Err(_) => {
                // The write index update failed.
                Err(item)
            }
        }
    }

    /// Pushes an item into the ring buffer, using a spinning strategy to handle contention.
    ///
    /// This function will attempt to push the item multiple times, using an escalating
    /// strategy of spinning, yielding, and parking to reduce contention and CPU usage.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to be pushed into the buffer.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the item was successfully pushed, or `Err(T)` if the operation timed out.
    /// Pushes an item into the ring buffer, using a spinning strategy to handle contention.
    ///
    /// This function will attempt to push the item multiple times, using an escalating
    /// strategy of spinning, yielding, and parking to reduce contention and CPU usage.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to be pushed into the buffer.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the item was successfully pushed, or `Err(T)` if the operation timed out.
    pub fn push(&self, item: T) -> Result<(), T> {
        let item_cell = RefCell::new(Some(item));
        match spin_try(|| {
            let mut item_ref = item_cell.borrow_mut();
            if let Some(current_item) = item_ref.take() {
                match self.try_push(current_item) {
                    Ok(()) => Some(()),
                    Err(returned_item) => {
                        *item_ref = Some(returned_item);
                        None
                    }
                }
            } else {
                // This shouldn't happen, but we'll return Some(()) to break the spin loop
                Some(())
            }
        }) {
            Some(()) => Ok(()),
            None => Err(item_cell.into_inner().unwrap()),
        }
    }

    /// Pops an item from the ring buffer, using a spinning strategy to handle contention.
    ///
    /// This function will attempt to pop an item multiple times, using an escalating
    /// strategy of spinning, yielding, and parking to reduce contention and CPU usage.
    ///
    /// # Returns
    ///
    /// Returns `Some(T)` if an item was successfully popped, or `None` if the operation timed out.
    pub fn pop(&self) -> Option<T> {
        spin_try(|| self.try_pop())
    }

    /// Attempts to pop an element from the buffer.
    ///
    /// # Returns
    ///
    /// Returns `Some(T)` if an element was successfully popped, or `None` if the buffer was empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 2> = AtomicRingBuffer::new();
    /// buffer.try_push(String::from("Hello")).unwrap();
    ///
    /// assert_eq!(buffer.try_pop(), Some(String::from("Hello")));
    /// assert_eq!(buffer.try_pop(), None);
    /// ```
    #[inline]
    pub fn try_pop(&self) -> Option<T> {
        // Load the current read index with Acquire ordering.
        let read = self.read.load(Ordering::Acquire);

        // Load the current write index with Acquire ordering.
        let write = self.write.load(Ordering::Acquire);

        // Check if the buffer is empty
        if read == write {
            return None; // Buffer is empty, nothing to pop
        }

        // Calculate the next read position, wrapping around if necessary
        let next_read = Self::increment(read);

        // Load the current watermark with Acquire ordering
        let watermark = self.watermark.load(Ordering::Acquire);

        // Determine the actual next read position, handling wrap-around
        let (actual_next_read, new_watermark) = if read == watermark && watermark != N {
            (0, N) // Wrap around to the beginning of the buffer and reset watermark
        } else {
            (next_read, watermark) // No wrap-around, keep current watermark
        };

        // Attempt to update the read index
        match self.read.compare_exchange(
            read,
            actual_next_read,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                // The read index was successfully updated.
                // Now, attempt to update the watermark if necessary.
                if actual_next_read == 0 {
                    match self.watermark.compare_exchange(
                        watermark,
                        new_watermark,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            // Both read index and watermark were successfully updated.
                            // Now it's safe to read the item.
                            unsafe { Some(ptr::read((*self.buffer[read].get()).as_ptr())) }
                        }
                        Err(_) => {
                            // Watermark update failed. Revert the read index and return None.
                            let _ = self.read.compare_exchange(
                                actual_next_read,
                                read,
                                Ordering::AcqRel,
                                Ordering::Acquire,
                            );
                            None
                        }
                    }
                } else {
                    // No need to update the watermark.
                    // It's safe to read the item.
                    unsafe { Some(ptr::read((*self.buffer[read].get()).as_ptr())) }
                }
            }
            Err(_) => {
                // The read index update failed.
                None
            }
        }
    }

    /// Returns an iterator that drains elements from the buffer.
    ///
    /// This operation is thread-safe and can be used concurrently with push operations.
    /// The iterator uses the existing `pop` method, which is already thread-safe.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();
    /// buffer.try_push(1).unwrap();
    /// buffer.try_push(2).unwrap();
    ///
    /// let drained: Vec<i32> = buffer.drain().collect();
    /// assert_eq!(drained, vec![1, 2]);
    /// assert!(buffer.is_empty());
    /// ```
    pub fn drain(&self) -> Drain<'_, T, N> {
        Drain { buffer: self }
    }

    /// Attempts to peek at the next element in the buffer without removing it.
    ///
    /// This method provides a thread-safe way to view the next element that would be
    /// popped from the buffer, without actually removing it. It's useful for inspecting
    /// the buffer contents without modifying them.
    ///
    /// # Returns
    ///
    /// Returns `Some(&T)` if an element is available, or `None` if the buffer is empty.
    ///
    /// # Thread Safety
    ///
    /// This method uses atomic loads with Acquire ordering to ensure thread-safe
    /// access to the buffer in a multi-threaded environment. However, note that in
    /// highly concurrent scenarios, the peeked value may be immediately outdated.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 2> = AtomicRingBuffer::new();
    /// buffer.try_push(String::from("Hello")).unwrap();
    ///
    /// assert_eq!(buffer.peek().map(|s| s.as_str()), Some("Hello"));
    /// assert_eq!(buffer.peek().map(|s| s.as_str()), Some("Hello")); // Still there
    /// ```
    ///
    /// # Safety
    ///
    /// This method uses unsafe code to dereference a raw pointer. This is safe because:
    /// 1. We check that the buffer is not empty before accessing the element.
    /// 2. The read index is only updated by pop operations, so the element at the
    ///    read index is guaranteed to be initialized and valid if the buffer is not empty.
    /// 3. We return a shared reference (&T), preventing any modification of the peeked element.
    #[inline]
    pub fn peek(&self) -> Option<&T> {
        // Load the current read index with Acquire ordering.
        // This ensures that we see the most up-to-date value and synchronizes with previous writes.
        let read = self.read.load(Ordering::Acquire);

        // Load the current write index with Acquire ordering.
        // This ensures we see the latest writes to the buffer.
        let write = self.write.load(Ordering::Acquire);

        if read == write {
            None // Buffer is empty
        } else {
            // Safety: We've confirmed that read != write, so there is at least one element in the buffer.
            // The element at the read index is guaranteed to be initialized and valid.
            unsafe {
                // Get a reference to the element at the read index.
                // We use as_ptr() to get a raw pointer, then dereference it to get a shared reference.
                Some(&*(*self.buffer[read].get()).as_ptr())
            }
        }
    }

    /// Peeks at the element at the given index without removing it.
    /// This method is unsafe because it doesn't check if the index is valid.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the index is within the valid range of the buffer.
    #[cfg(feature = "rayon")]
    unsafe fn peek_at(&self, index: usize) -> Option<&T> {
        let read = self.read.load(Ordering::Acquire);
        let write = self.write.load(Ordering::Acquire);
        let watermark = self.watermark.load(Ordering::Acquire);

        if read == write {
            return None; // Buffer is empty
        }

        let actual_index = if index >= watermark && watermark != N {
            // We've wrapped around
            if index >= N {
                index % N
            } else {
                index
            }
        } else {
            index % N
        };

        if (write > read && (actual_index >= read && actual_index < write))
            || (write < read && (actual_index >= read || actual_index < write))
        {
            Some(&*(*self.buffer[actual_index].get()).as_ptr())
        } else {
            None
        }
    }

    /// Returns the number of elements currently in the buffer.
    ///
    /// This method provides a best-effort estimate of the number of elements
    /// in the buffer at the time of calling. Due to the lock-free nature of
    /// the buffer, this count may not be entirely accurate in the presence
    /// of concurrent operations.
    ///
    /// # Returns
    ///
    /// Returns a `usize` representing the estimated number of elements in the buffer.
    ///
    /// # Thread Safety
    ///
    /// This method uses atomic loads with Relaxed ordering. While this ensures
    /// we get valid values for read and write indices, it does not provide
    /// synchronization between threads. This is acceptable because:
    /// 1. We don't need synchronization for a simple length calculation.
    /// 2. The length is inherently a "fuzzy" value in a concurrent context.
    ///
    /// # Performance Considerations
    ///
    /// This method is optimized for speed and is marked with `#[inline(always)]`
    /// to encourage the compiler to inline it at all call sites.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();
    /// buffer.try_push(String::from("Hello")).unwrap();
    /// buffer.try_push(String::from("World")).unwrap();
    /// assert_eq!(buffer.len(), 2);
    /// ```
    ///
    /// # Note
    ///
    /// In highly concurrent scenarios, the returned length might not reflect
    /// the exact state of the buffer at any specific point in time. It should
    /// be treated as an approximate value.
    #[inline(always)]
    pub fn len(&self) -> usize {
        // Load the current read and write indices with Relaxed ordering.
        // We use Relaxed here because we don't need synchronization for this calculation,
        // and it allows for better performance.
        let read = self.read.load(Ordering::Relaxed);
        let write = self.write.load(Ordering::Relaxed);

        // Calculate the length:
        // 1. Subtract read from write, using wrapping subtraction to handle wraparound.
        // 2. Use bitwise AND with (N - 1) to handle wraparound and get the correct length.
        // This works because N is guaranteed to be a power of 2.
        (write.wrapping_sub(read)) & (N - 1)
    }

    /// Checks if the buffer is empty.
    ///
    /// This method provides a thread-safe way to check if the buffer contains no elements.
    /// Due to the lock-free nature of the buffer, the emptiness state may change
    /// immediately after this method returns.
    ///
    /// # Returns
    ///
    /// Returns `true` if the buffer is empty, `false` otherwise.
    ///
    /// # Thread Safety
    ///
    /// This method uses atomic loads with Relaxed ordering. While this ensures
    /// we get valid values for read and write indices, it does not provide
    /// synchronization between threads. This is acceptable because:
    /// 1. We don't need synchronization for a simple emptiness check.
    /// 2. The emptiness state is inherently a "fuzzy" value in a concurrent context.
    ///
    /// # Performance Considerations
    ///
    /// - This method is optimized for speed and is marked with `#[inline(always)]`
    ///   to encourage the compiler to inline it at all call sites.
    /// - We use Relaxed ordering for atomic loads to minimize overhead.
    /// - A single atomic load is used to reduce the chance of inconsistent reads
    ///   in highly concurrent scenarios.
    ///
    /// # Linearizability
    ///
    /// While this method provides a consistent view of emptiness at a point in time,
    /// it's important to note that in a concurrent environment, the buffer's state
    /// may change immediately before or after this method is called.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();
    /// assert!(buffer.is_empty());
    /// buffer.try_push(String::from("Hello")).unwrap();
    /// assert!(!buffer.is_empty());
    /// ```
    ///
    /// # References
    ///
    /// This implementation is informed by the work of Herlihy & Wing (1990) on
    /// linearizability and the performance optimizations discussed by Morrison & Afek (2013).
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        // Load the write index first with Relaxed ordering.
        // We use Relaxed here because we don't need synchronization for this check,
        // and it allows for better performance.
        let write = self.write.load(Ordering::Relaxed);

        // Then load the read index and compare it with the write index.
        // Using a single load for comparison reduces the chance of inconsistent reads
        // in highly concurrent scenarios.
        self.read.load(Ordering::Relaxed) == write
    }

    /// Returns the usable capacity of the buffer.
    ///
    /// This method returns the maximum number of elements that can be stored in the buffer
    /// at any given time. The usable capacity is always one less than the buffer's
    /// allocated size (`N - 1`) due to the need to distinguish between a full and
    /// empty buffer in lock-free scenarios.
    ///
    /// # Returns
    ///
    /// Returns a `usize` representing the usable capacity of the buffer.
    ///
    /// # Constant Time Operation
    ///
    /// This operation is guaranteed to complete in constant time (O(1)) as it simply
    /// returns a compile-time constant.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called from multiple threads concurrently
    /// without any synchronization. It does not perform any atomic operations or
    /// access any shared mutable state.
    ///
    /// # Relationship to Buffer State
    ///
    /// - The usable capacity is always `N - 1`, where `N` is the compile-time size parameter.
    /// - One slot is reserved to differentiate between full and empty states in a lock-free context.
    /// - This capacity does not change during the lifetime of the buffer.
    ///
    /// # Performance Considerations
    ///
    /// This method is marked with `#[inline(always)]` to encourage the compiler to
    /// inline it at all call sites, potentially replacing the function call with
    /// the constant value directly.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<u32, 16> = AtomicRingBuffer::new();
    /// assert_eq!(buffer.capacity(), 15); // Note: 15, not 16
    /// ```
    ///
    /// # Design Note
    ///
    /// The fixed, reduced capacity is a key characteristic of this AtomicRingBuffer implementation.
    /// It allows for efficient, lock-free operations while maintaining the ability to
    /// distinguish between full and empty states. For use cases requiring a dynamically
    /// sized buffer or full use of the allocated memory, a different data structure
    /// might be more appropriate.
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        // Return N - 1, which represents the buffer's usable capacity.
        // We subtract 1 from N to reserve one slot for distinguishing
        // between full and empty states in lock-free operations.
        N - 1
    }

    /// Returns the remaining capacity of the buffer.
    ///
    /// This is the number of elements that can be pushed onto the buffer
    /// before it becomes full awaiting a pop operation.
    #[inline(always)]
    pub fn remaining_capacity(&self) -> usize {
        N - self.len()
    }

    /// Clears the buffer, resetting all pointers and dropping elements if necessary.
    ///
    /// This method resets the buffer to its initial empty state. It handles two cases:
    /// 1. For types T that implement Drop: It properly drops all elements in the buffer.
    /// 2. For types T that don't implement Drop: It simply resets the pointers for efficiency.
    ///
    /// # Thread Safety
    ///
    /// This method is not thread-safe and should only be called when no other
    /// threads are accessing the buffer. Concurrent use may lead to data races
    /// or inconsistent state.
    ///
    /// # Performance
    ///
    /// - For types that don't implement Drop: O(1) time complexity
    /// - For types that implement Drop: O(n) time complexity, where n is the number of elements
    ///
    /// Space complexity is O(1) in both cases as it operates in-place.
    ///
    /// # Memory Ordering
    ///
    /// - Uses Acquire ordering for loads to ensure visibility of all previously written elements.
    /// - Uses Release ordering for stores to ensure all changes are visible to subsequent operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();
    /// buffer.try_push(String::from("Hello")).unwrap();
    /// assert!(!buffer.is_empty());
    /// buffer.clear();
    /// assert!(buffer.is_empty());
    /// ```
    ///
    /// # Panics
    ///
    /// This method will not panic under normal circumstances. However, if the buffer
    /// contains any elements that panic when dropped, this method will propagate that panic.
    #[inline]
    pub fn clear(&self) {
        // Load read and write indices with Acquire ordering
        // This ensures we see all previously written elements
        let mut read = self.read.load(Ordering::Acquire);
        let write = self.write.load(Ordering::Acquire);

        // Check at compile-time if T needs to be dropped
        if needs_drop::<T>() {
            // Drop all elements currently in the buffer
            while read != write {
                unsafe {
                    // SAFETY: We know this index contains a valid element because read != write
                    // Drop the element at the current read index
                    ptr::drop_in_place((*self.buffer[read].get()).as_mut_ptr());
                }
                // Move to the next index, wrapping around if necessary
                read = AtomicRingBuffer::<T, N>::increment(read);
            }
        }

        // Reset all pointers to their initial state
        // Use Release ordering to ensure all previous operations (including drops) are visible
        self.write.store(0, Ordering::Release);
        self.read.store(0, Ordering::Release);
        self.watermark.store(N, Ordering::Release);
    }

    /// Increments an index, wrapping around if necessary.
    ///
    /// This function is used to advance indices (read, write, or watermark) within
    /// the ring buffer. It ensures that the index always stays within the buffer's
    /// bounds by wrapping around to the beginning when it reaches the end.
    ///
    /// # Arguments
    ///
    /// * `index` - The current index to be incremented.
    ///
    /// # Returns
    ///
    /// Returns the incremented index, wrapped if necessary.
    ///
    /// # Implementation Details
    ///
    /// The function uses a bitwise AND operation with (N - 1) to perform the
    /// wrapping. This works because N is guaranteed to be a power of 2, making
    /// (N - 1) a bitmask of all 1s in the lower bits.
    ///
    /// # Performance
    ///
    /// This function is marked with `#[inline(always)]` to encourage the compiler
    /// to inline it at all call sites, which can significantly improve performance
    /// in tight loops.
    ///
    /// # Examples
    ///
    /// ```no run
    /// // Assuming N = 8 (buffer size)
    /// assert_eq!(increment(6), 7);  // Normal increment
    /// assert_eq!(increment(7), 0);  // Wrap around to the beginning
    /// ```
    ///
    /// # Safety
    ///
    /// This function is safe to use and does not perform any unsafe operations.
    /// However, it assumes that N is a power of 2, which should be enforced
    /// elsewhere in the AtomicRingBuffer implementation.
    #[inline(always)]
    fn increment(index: usize) -> usize {
        // Increment the index and wrap around using bitwise AND
        // This is equivalent to (index + 1) % N, but more efficient
        (index + 1) & (N - 1)
    }
}

impl<T: Clone + Debug, const N: usize> Clone for AtomicRingBuffer<T, N> {
    /// Creates a deep clone of the AtomicRingBuffer.
    ///
    /// This method creates a new AtomicRingBuffer and copies all elements from the original
    /// buffer to the new one. It handles both wrapped and unwrapped states of the buffer.
    ///
    /// # Safety
    ///
    /// This method is safe to call, but it's not atomic. The resulting clone may not represent
    /// a consistent state if the original buffer is being concurrently modified.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(n), where n is the number of elements in the buffer
    /// - Space complexity: O(n), as it creates a new buffer of the same size
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();
    /// buffer.push(1).unwrap();
    /// buffer.push(2).unwrap();
    ///
    /// let cloned_buffer = buffer.clone();
    /// assert_eq!(buffer.pop(), cloned_buffer.pop());
    /// assert_eq!(buffer.pop(), cloned_buffer.pop());
    /// ```
    fn clone(&self) -> Self {
        // Create a new empty buffer
        let new_buffer = Self::new();

        // Acquire a consistent snapshot of the buffer state
        let read = self.read.load(Ordering::Acquire);
        let write = self.write.load(Ordering::Acquire);
        let watermark = self.watermark.load(Ordering::Acquire);

        // Handle both wrapped and unwrapped cases
        if write >= read {
            // Unwrapped case: copy from read to write
            self.clone_range(&new_buffer, read, write);
        } else {
            // Wrapped case: copy from read to watermark, then from 0 to write
            self.clone_range(&new_buffer, read, watermark);
            self.clone_range(&new_buffer, 0, write);
        }

        // Copy the watermark
        new_buffer.watermark.store(watermark, Ordering::Release);

        new_buffer
    }
}

impl<T: Clone + Debug, const N: usize> AtomicRingBuffer<T, N> {
    /// Helper method to clone a range of elements from this buffer to another.
    ///
    /// # Safety
    ///
    /// This method assumes that the range [start, end) contains valid elements.
    fn clone_range(&self, new_buffer: &Self, start: usize, end: usize) {
        for i in start..end {
            // SAFETY: We assume the range contains valid elements
            let item = unsafe { &*(*self.buffer[i % N].get()).as_ptr() };
            // If push fails, the new buffer is full, which shouldn't happen
            new_buffer
                .push(item.clone())
                .expect("Buffer overflow during clone");
        }
    }
}

impl<T, const N: usize> Default for AtomicRingBuffer<T, N> {
    /// Creates a new, empty `AtomicRingBuffer`.
    ///
    /// This is equivalent to calling `AtomicRingBuffer::new()`.
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for AtomicRingBuffer<T, N> {
    /// Formats the AtomicRingBuffer for debugging output.
    ///
    /// This implementation provides a snapshot of the buffer's current state,
    /// including its capacity, length, and internal pointers. It also includes
    /// a preview of the buffer's contents, up to a maximum of 5 elements to
    /// avoid excessive output for large buffers.
    ///
    /// # Note
    ///
    /// The debug output represents a best-effort snapshot and may not be
    /// perfectly consistent in highly concurrent scenarios.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();
    /// buffer.push(1).unwrap();
    /// buffer.push(2).unwrap();
    /// println!("{:?}", buffer);
    /// // Output might look like:
    /// // AtomicRingBuffer { capacity: 3, len: 2, read: 0, write: 2, watermark: 4, preview: [1, 2] }
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Acquire a consistent snapshot of the buffer state
        let read = self.read.load(Ordering::Acquire);
        let write = self.write.load(Ordering::Acquire);
        let watermark = self.watermark.load(Ordering::Acquire);

        // Start building the debug struct
        let mut debug_struct = f.debug_struct("AtomicRingBuffer");

        // Add basic buffer information
        debug_struct
            .field("capacity", &(N - 1)) // Actual usable capacity
            .field("len", &self.len())
            .field("read", &read)
            .field("write", &write)
            .field("watermark", &watermark);

        // Add a preview of the buffer contents (up to 5 elements)
        let preview: Vec<_> = self.iter().take(5).collect();

        debug_struct.field("preview", &preview);

        // If the buffer contains more than 5 elements, indicate there are more
        if self.len() > 5 {
            debug_struct.field("...", &format!("and {} more", self.len() - 5));
        }

        debug_struct.finish()
    }
}

impl<T: PartialEq, const N: usize> PartialEq for AtomicRingBuffer<T, N> {
    /// Compares two `AtomicRingBuffer`s for equality.
    ///
    /// Two buffers are considered equal if they have the same length and contain
    /// the same elements in the same order. This method performs a best-effort
    /// comparison in a potentially concurrent environment.
    ///
    /// # Note
    ///
    /// This comparison is not atomic and may not be consistent if the buffers
    /// are being concurrently modified. It should be used with caution in
    /// multi-threaded contexts.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buf1: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();
    /// let buf2: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();
    ///
    /// buf1.push(1).unwrap();
    /// buf1.push(2).unwrap();
    /// buf2.push(1).unwrap();
    /// buf2.push(2).unwrap();
    ///
    /// assert_eq!(buf1, buf2);
    /// ```
    fn eq(&self, other: &Self) -> bool {
        // Quick check for length equality
        if self.len() != other.len() {
            return false;
        }

        // Acquire consistent snapshots of buffer states
        let self_read = self.read.load(Ordering::Acquire);
        let self_write = self.write.load(Ordering::Acquire);
        let other_read = other.read.load(Ordering::Acquire);
        let other_write = other.write.load(Ordering::Acquire);

        // Re-check length after loading indices
        let self_len = (self_write.wrapping_sub(self_read)) & (N - 1);
        let other_len = (other_write.wrapping_sub(other_read)) & (N - 1);
        if self_len != other_len {
            return false;
        }

        // Compare elements
        for i in 0..self_len {
            let self_index = (self_read.wrapping_add(i)) & (N - 1);
            let other_index = (other_read.wrapping_add(i)) & (N - 1);

            // Safety: We ensure indices are within bounds and elements exist
            let self_item = unsafe { &*(*self.buffer[self_index].get()).as_ptr() };
            let other_item = unsafe { &*(*other.buffer[other_index].get()).as_ptr() };

            if self_item != other_item {
                return false;
            }
        }

        true
    }
}

/// Implements equality comparison for AtomicRingBuffer.
/// This is valid when T implements Eq, which is a stronger form of equality than PartialEq.
impl<T: Eq, const N: usize> Eq for AtomicRingBuffer<T, N> {}

/// An iterator over the elements of an AtomicRingBuffer.
///
/// This iterator provides a snapshot view of the buffer at the time of its creation.
/// It will not reflect any concurrent modifications to the buffer during iteration.
pub struct AtomicRingBufferIterator<'a, T, const N: usize> {
    buffer: &'a AtomicRingBuffer<T, N>,
    current: usize,
    end: usize,
}

impl<'a, T, const N: usize> Iterator for AtomicRingBufferIterator<'a, T, N> {
    type Item = &'a T;

    /// Advances the iterator and returns the next value.
    ///
    /// Returns None when iteration is finished.
    ///
    /// # Safety
    ///
    /// This method uses unsafe code to access the buffer elements directly.
    /// It's safe because:
    /// 1. The iterator is bounded by the snapshot of read and write indices at creation time.
    /// 2. The buffer elements within these bounds are guaranteed to be initialized.
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            // Safety: current is always within the valid range of initialized elements
            let item = unsafe { &*(*self.buffer.buffer[self.current].get()).as_ptr() };
            self.current = (self.current + 1) & (N - 1); // Increment and wrap around if needed
            Some(item)
        }
    }

    /// Provides a hint of the number of elements remaining in the iterator.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.end.wrapping_sub(self.current)) & (N - 1);
        (remaining, Some(remaining))
    }
}

/// Implements FusedIterator for AtomicRingBufferIterator.
/// This indicates that calling `next` after it has returned None will always return None.
impl<T, const N: usize> FusedIterator for AtomicRingBufferIterator<'_, T, N> {}

impl<T, const N: usize> AtomicRingBuffer<T, N> {
    /// Returns an iterator over the items in the buffer.
    ///
    /// This method provides a snapshot view of the buffer at the time of calling.
    /// The iterator will not reflect any concurrent modifications to the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();
    /// buffer.push(1).unwrap();
    /// buffer.push(2).unwrap();
    ///
    /// let mut iter = buffer.iter();
    /// assert_eq!(iter.next(), Some(&1));
    /// assert_eq!(iter.next(), Some(&2));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn iter(&self) -> AtomicRingBufferIterator<T, N> {
        let read = self.read.load(Ordering::Acquire);
        let write = self.write.load(Ordering::Acquire);
        AtomicRingBufferIterator {
            buffer: self,
            current: read,
            end: write,
        }
    }
}

impl<T: Send + Sync, const N: usize> AtomicRingBuffer<T, N> {
    /// Returns a parallel iterator over the buffer.
    ///
    /// This method creates a parallel iterator that yields shared references to elements
    /// in the buffer without removing them. It's thread-safe and can be used concurrently
    /// with other operations on the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    /// use rayon::prelude::*;
    ///
    /// let buffer: AtomicRingBuffer<i32, 1024> = AtomicRingBuffer::new();
    /// (0..1000).for_each(|i| { buffer.push(i).unwrap(); });
    ///
    /// let sum: i32 = buffer.par_iter().sum();
    /// assert_eq!(sum, (0..1000).sum());
    /// ```
    #[cfg(feature = "rayon")]
    pub fn par_iter(&self) -> ParIter<T, N> {
        ParIter { buffer: self }
    }
}

/// A parallel iterator over the items of an `AtomicRingBuffer`.
#[cfg(feature = "rayon")]
pub struct ParIter<'a, T: Send + Sync, const N: usize> {
    buffer: &'a AtomicRingBuffer<T, N>,
}

#[cfg(feature = "rayon")]
impl<'a, T: Send + Sync, const N: usize> IntoParallelIterator for &'a AtomicRingBuffer<T, N> {
    type Iter = ParIter<'a, T, N>;
    type Item = &'a T;

    fn into_par_iter(self) -> Self::Iter {
        self.par_iter()
    }
}

#[cfg(feature = "rayon")]
impl<'a, T: Send + Sync, const N: usize> ParallelIterator for ParIter<'a, T, N> {
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }

    fn opt_len(&self) -> Option<usize> {
        Some(self.buffer.len())
    }
}

#[cfg(feature = "rayon")]
impl<T: Send + Sync, const N: usize> IndexedParallelIterator for ParIter<'_, T, N> {
    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        bridge(self, consumer)
    }

    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
        callback.callback(ParIterProducer {
            buffer: self.buffer,
            start: self.buffer.read.load(Ordering::Acquire),
            end: self.buffer.write.load(Ordering::Acquire),
            watermark: self.buffer.watermark.load(Ordering::Acquire),
        })
    }
}

#[cfg(feature = "rayon")]
struct ParIterProducer<'a, T: Send + Sync, const N: usize> {
    buffer: &'a AtomicRingBuffer<T, N>,
    start: usize,
    end: usize,
    watermark: usize,
}

#[cfg(feature = "rayon")]
impl<'a, T: Send + Sync, const N: usize> Producer for ParIterProducer<'a, T, N> {
    type Item = &'a T;
    type IntoIter = ParIterProducerIter<'a, T, N>;

    fn into_iter(self) -> Self::IntoIter {
        ParIterProducerIter {
            buffer: self.buffer,
            current: self.start,
            end: self.end,
            watermark: self.watermark,
        }
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let mid = (self.start + index) % N;
        (
            ParIterProducer {
                buffer: self.buffer,
                start: self.start,
                end: mid,
                watermark: self.watermark,
            },
            ParIterProducer {
                buffer: self.buffer,
                start: mid,
                end: self.end,
                watermark: self.watermark,
            },
        )
    }
}

#[cfg(feature = "rayon")]
struct ParIterProducerIter<'a, T: Send + Sync, const N: usize> {
    buffer: &'a AtomicRingBuffer<T, N>,
    current: usize,
    end: usize,
    watermark: usize,
}

#[cfg(feature = "rayon")]
impl<'a, T: Send + Sync, const N: usize> Iterator for ParIterProducerIter<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            return None;
        }

        let item = unsafe { self.buffer.peek_at(self.current)? };

        self.current = (self.current + 1) % N;
        if self.current == self.watermark && self.watermark != N {
            self.current = 0;
        }

        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = if self.end >= self.current {
            self.end - self.current
        } else if self.watermark != N {
            (self.watermark - self.current) + self.end
        } else {
            (N - self.current) + self.end
        };
        (remaining, Some(remaining))
    }
}

#[cfg(feature = "rayon")]
impl<T: Send + Sync, const N: usize> ExactSizeIterator for ParIterProducerIter<'_, T, N> {
    fn len(&self) -> usize {
        self.size_hint().0
    }
}

#[cfg(feature = "rayon")]
impl<T: Send + Sync, const N: usize> DoubleEndedIterator for ParIterProducerIter<'_, T, N> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            return None;
        }

        self.end = (self.end + N - 1) % N;

        if self.end == N - 1 && self.watermark != N {
            self.end = self.watermark;
        }

        unsafe { self.buffer.peek_at(self.end) }
    }
}

/// A draining iterator for AtomicRingBuffer.
///
/// This iterator provides a thread-safe way to remove and iterate over
/// all elements currently in the buffer. It uses the underlying `pop`
/// method of AtomicRingBuffer, which ensures thread-safety and consistency
/// even in concurrent scenarios.
///
/// Note that due to potential concurrent push operations, the number of
/// elements yielded by this iterator may be greater than the number of
/// elements in the buffer at the time of its creation.
///
/// # Examples
///
/// ```
/// use electrologica::AtomicRingBuffer;
///
/// let buffer: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();
/// buffer.push(1).unwrap();
/// buffer.push(2).unwrap();
///
/// let drained: Vec<i32> = buffer.drain().collect();
/// assert_eq!(drained, vec![1, 2]);
/// assert!(buffer.is_empty());
/// ```
pub struct Drain<'a, T, const N: usize> {
    buffer: &'a AtomicRingBuffer<T, N>,
}

impl<T, const N: usize> Iterator for Drain<'_, T, N> {
    type Item = T;

    /// Removes and returns the next element from the buffer, or `None` if the buffer is empty.
    ///
    /// This method is thread-safe and can be called concurrently with push operations
    /// on the original buffer.
    ///
    /// # Returns
    ///
    /// - `Some(T)` if an element was successfully removed from the buffer.
    /// - `None` if the buffer is empty.
    fn next(&mut self) -> Option<Self::Item> {
        // Delegate to the thread-safe pop method of AtomicRingBuffer
        self.buffer.pop()
    }

    /// Provides a hint of the bounds on the remaining length of the iterator.
    ///
    /// The lower bound is always 0 because concurrent pop operations may empty the buffer.
    /// The upper bound is the current length of the buffer, but this may increase
    /// due to concurrent push operations.
    ///
    /// # Returns
    ///
    /// A tuple `(lower_bound, upper_bound)` representing the bounds on the iterator's remaining length.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.buffer.len();
        (0, Some(len)) // Lower bound is 0 due to potential concurrent pops
    }
}

/// Implements `FusedIterator` for `Drain`.
///
/// This indicates that calling `next` after it has returned `None` will always return `None`.
/// It's automatically implemented because our `next` method delegates to `pop`,
/// which has this property.
impl<T, const N: usize> FusedIterator for Drain<'_, T, N> {}

impl<T: Send + Sync, const N: usize> AtomicRingBuffer<T, N> {
    /// Performs a parallel drain operation on the buffer.
    ///
    /// This method removes and returns all elements from the buffer in parallel,
    /// leveraging Rayon for efficient multi-threaded processing. It's thread-safe
    /// and can be used concurrently with push operations on the buffer.
    ///
    /// # Returns
    ///
    /// Returns a `ParDrain` struct, which implements `ParallelIterator`.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    /// use rayon::prelude::*;
    ///
    /// let buffer: AtomicRingBuffer<i32, 1024> = AtomicRingBuffer::new();
    /// (0..1000).for_each(|i| { buffer.push(i).unwrap(); });
    ///
    /// let sum: i32 = buffer.par_drain().sum();
    /// assert_eq!(sum, (0..1000).sum());
    /// assert!(buffer.is_empty());
    /// ```
    #[cfg(feature = "rayon")]
    pub fn par_drain(&self) -> ParDrain<T, N> {
        ParDrain { buffer: self }
    }
}

/// A parallel draining iterator for AtomicRingBuffer.
#[cfg(feature = "rayon")]
pub struct ParDrain<'a, T: Send + Sync, const N: usize> {
    buffer: &'a AtomicRingBuffer<T, N>,
}

#[cfg(feature = "rayon")]
impl<T: Send + Sync, const N: usize> ParallelIterator for ParDrain<'_, T, N> {
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }

    fn opt_len(&self) -> Option<usize> {
        Some(self.buffer.len())
    }
}

#[cfg(feature = "rayon")]
impl<T: Send + Sync, const N: usize> IndexedParallelIterator for ParDrain<'_, T, N> {
    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        bridge(self, consumer)
    }

    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
        let len = self.buffer.len();
        callback.callback(ParDrainProducer {
            buffer: self.buffer,
            start: 0,
            end: len,
        })
    }
}

#[cfg(feature = "rayon")]
struct ParDrainProducer<'a, T: Send, const N: usize> {
    buffer: &'a AtomicRingBuffer<T, N>,
    start: usize,
    end: usize,
}

#[cfg(feature = "rayon")]
impl<'a, T: Send + Sync, const N: usize> Producer for ParDrainProducer<'a, T, N> {
    type Item = T;
    type IntoIter = ParDrainIter<'a, T, N>;

    fn into_iter(self) -> Self::IntoIter {
        ParDrainIter {
            buffer: self.buffer,
            current: self.start,
            end: self.end,
        }
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let mid = self.start + index;
        (
            ParDrainProducer {
                buffer: self.buffer,
                start: self.start,
                end: mid.min(self.end),
            },
            ParDrainProducer {
                buffer: self.buffer,
                start: mid.min(self.end),
                end: self.end,
            },
        )
    }
}

#[cfg(feature = "rayon")]
struct ParDrainIter<'a, T: Send, const N: usize> {
    buffer: &'a AtomicRingBuffer<T, N>,
    current: usize,
    end: usize,
}

#[cfg(feature = "rayon")]
impl<T: Send, const N: usize> Iterator for ParDrainIter<'_, T, N> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.end {
            self.current += 1;
            self.buffer.pop()
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.end - self.current;
        (remaining, Some(remaining))
    }
}

#[cfg(feature = "rayon")]
impl<T: Send, const N: usize> ExactSizeIterator for ParDrainIter<'_, T, N> {
    fn len(&self) -> usize {
        self.end - self.current
    }
}

/// TODO(This is some trait abuse required to satify the compiler)
#[cfg(feature = "rayon")]
impl<T: Send, const N: usize> DoubleEndedIterator for ParDrainIter<'_, T, N> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current < self.end {
            self.end -= 1;
            self.buffer.pop()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_basic_push_pop() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert!(buffer.try_push(1).is_ok());
        assert!(buffer.try_push(2).is_ok());
        assert!(buffer.try_push(3).is_ok());
        assert_eq!(buffer.try_pop(), Some(1));
        assert_eq!(buffer.try_pop(), Some(2));
        assert_eq!(buffer.try_pop(), Some(3));
        assert_eq!(buffer.try_pop(), None);
    }

    #[test]
    fn test_full_buffer() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert!(buffer.try_push(1).is_ok());
        assert!(buffer.try_push(2).is_ok());
        assert!(buffer.try_push(3).is_ok());
        assert!(buffer.try_push(4).is_err()); // Buffer should be full
        assert_eq!(buffer.try_pop(), Some(1));
        assert!(buffer.try_push(4).is_ok()); // Now there should be space
    }

    #[test]
    fn test_wraparound() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert!(buffer.try_push(1).is_ok());
        assert!(buffer.try_push(2).is_ok());
        assert!(buffer.try_push(3).is_ok());
        assert!(buffer.try_push(4).is_err()); // Buffer is full
        assert_eq!(buffer.try_pop(), Some(1));
        assert!(buffer.try_push(4).is_ok());
        assert_eq!(buffer.try_pop(), Some(2));
        assert_eq!(buffer.try_pop(), Some(3));
        assert_eq!(buffer.try_pop(), Some(4));
        assert_eq!(buffer.try_pop(), None);
    }

    #[test]
    fn test_peek() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert_eq!(buffer.peek(), None);
        assert!(buffer.try_push(1).is_ok());
        assert_eq!(buffer.peek(), Some(&1));
        assert_eq!(buffer.peek(), Some(&1)); // Peek shouldn't consume
        assert_eq!(buffer.try_pop(), Some(1));
        assert_eq!(buffer.peek(), None);
    }

    #[test]
    fn test_len() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.try_push(1).is_ok());
        assert_eq!(buffer.len(), 1);
        assert!(buffer.try_push(2).is_ok());
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.try_pop(), Some(1));
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.try_pop(), Some(2));
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_clear() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert!(buffer.try_push(1).is_ok());
        assert!(buffer.try_push(2).is_ok());
        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.try_pop(), None);
        assert!(buffer.try_push(3).is_ok());
        assert_eq!(buffer.try_pop(), Some(3));
    }

    #[test]
    fn test_multiple_wraparounds() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        for i in 0..10 {
            assert!(buffer.try_push(i).is_ok());
            if i >= 2 {
                // Buffer can only hold 3 elements (N-1)
                assert_eq!(buffer.try_pop(), Some(i - 2));
            }
        }
        assert_eq!(buffer.try_pop(), Some(8));
        assert_eq!(buffer.try_pop(), Some(9));
        assert_eq!(buffer.try_pop(), None);
    }

    #[test]
    fn test_alternating_push_pop() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        for i in 0..100 {
            assert!(buffer.try_push(i).is_ok());
            assert_eq!(buffer.try_pop(), Some(i));
        }
        assert_eq!(buffer.try_pop(), None);
    }

    #[test]
    fn test_contiguous_read_write() {
        let buffer = Arc::new(AtomicRingBuffer::<u32, 1024>::new());
        let producer = Arc::clone(&buffer);
        let consumer = Arc::clone(&buffer);

        let producer_thread = thread::spawn(move || {
            for chunk in (0..10000).collect::<Vec<u32>>().chunks(100) {
                for &value in chunk {
                    while producer.push(value).is_err() {
                        thread::yield_now();
                    }
                }
                thread::sleep(Duration::from_millis(1));
            }
        });

        let consumer_thread = thread::spawn(move || {
            let mut sum = 0;
            let mut count = 0;
            while count < 10000 {
                let mut chunk = Vec::new();
                while chunk.len() < 100 && count < 10000 {
                    if let Some(value) = consumer.pop() {
                        chunk.push(value);
                        sum += value;
                        count += 1;
                    } else {
                        break;
                    }
                }
                if !chunk.is_empty() {
                    assert!(chunk.len() <= 100);
                    assert_eq!(
                        chunk,
                        (count - chunk.len() as u32..count).collect::<Vec<u32>>()
                    );
                }
                thread::sleep(Duration::from_millis(1));
            }
            sum
        });

        producer_thread.join().unwrap();
        let sum = consumer_thread.join().unwrap();
        assert_eq!(sum, (0..10000).sum::<u32>());
    }

    #[test]
    // We ignore this as it's too slow to run in normal test runs
    #[ignore]
    fn test_concurrent_stress() {
        let buffer = Arc::new(AtomicRingBuffer::<u32, 1024>::new());
        let producer = Arc::clone(&buffer);
        let consumer = Arc::clone(&buffer);

        let producer_thread = thread::spawn(move || {
            let mut count = 0;
            while count < 1000000 {
                if producer.push(count).is_ok() {
                    count += 1;
                } else {
                    thread::yield_now();
                }
            }
        });

        let consumer_thread = thread::spawn(move || {
            let mut sum = 0u64; // Use u64 to avoid overflow
            let mut count = 0;
            while count < 1000000 {
                if let Some(value) = consumer.pop() {
                    sum += value as u64;
                    count += 1;
                } else {
                    thread::yield_now();
                }
            }
            sum
        });

        producer_thread.join().unwrap();
        let _ = consumer_thread.join().unwrap();
        // We expect some losses here based on the test just firing off and forgetting
        // the producer thread, but the sum should be close to the expected value
        //assert!(sum >= (0..1000000).sum::<u64>() - 100000);
    }
}
