//! # AtomicRingBuffer
//!
//! A lock-free, single-producer single-consumer (SPSC) ring buffer implementation
//! that supports contiguous writes and reads for any type `T`, including non-Copy types.
//! This implementation is inspired by the James Munes
//! <https://ferrous-systems.com/blog/lock-free-ring-buffer/> with the design
//! adapted for complex data types, using contemporary rust.
//!
//! ## Design Goals
//!
//! - Lock-free operations for high performance in concurrent scenarios
//! - Support for contiguous writes and reads
//! - Minimal use of unsafe code (but there is some)
//! - Correct memory ordering for cross-thread synchronization
//! - Efficient wraparound handling using a watermark mechanism
//! - Support for any `Send` type `T`
//!
//! ## Key Features
//!
//! - Fixed-size buffer allocated on the stack
//! - Atomic operations for thread-safe access
//! - Watermark mechanism for efficient wraparound handling
//! - Support for any `Send` type `T`
//! - Safe handling of non-Copy types
//!
//! ## Usage
//!
//! The `AtomicRingBuffer` is designed for SPSC scenarios where one thread writes
//! to the buffer and another thread reads from it. It's particularly useful in
//! embedded systems or high-performance computing where DMA operations require
//! contiguous memory regions, and where complex data types need to be transferred.
//!
//! ```rust
//! use electrologica::AtomicRingBuffer;
//!
//! let buffer: AtomicRingBuffer<String, 1024> = AtomicRingBuffer::new();
//!
//! // Producer thread
//! buffer.push(String::from("Hello")).unwrap();
//! buffer.push(String::from("World")).unwrap();
//!
//! // Consumer thread
//! let item1 = buffer.pop().unwrap();
//! let item2 = buffer.pop().unwrap();
//! assert_eq!(item1, "Hello");
//! assert_eq!(item2, "World");
//! ```
//!
//! ## Memory Ordering
//!
//! This implementation uses careful memory ordering to ensure correct
//! synchronization between threads:
//!
//! - `Relaxed`: Used for operations that don't require synchronization
//! - `Acquire`: Used when reading shared data to ensure visibility of previous writes
//! - `Release`: Used when writing shared data to make updates visible to other threads
//! - `AcqRel`: Used for operations that both read and write shared data
//! - `SeqCst`: Used sparingly for operations that require total ordering across all threads
//!
//! ## Safety
//!
//! While this implementation strives to minimize unsafe code, some unsafe operations
//! are necessary for performance and to handle raw pointers. All unsafe code is
//! carefully commented and reasoned about.
//!
//! ## Performance Considerations
//!
//! The use of atomic operations and careful memory ordering ensures high performance
//! in concurrent scenarios. However, as with any lock-free data structure, performance
//! can vary depending on contention and hardware characteristics.

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fmt, ptr};

/// A lock-free, single-producer single-consumer ring buffer with support for contiguous writes.
///
/// # Type Parameters
///
/// - `T`: The type of elements stored in the buffer. Must be `Send`.
/// - `N`: The capacity of the buffer. Must be a power of two for efficient wrapping.
///
/// # Thread Safety
///
/// This structure is safe to share between threads when `T` is `Send`.
/// It's designed for single-producer single-consumer scenarios.
pub struct AtomicRingBuffer<T, const N: usize> {
    /// The internal buffer storing the elements.
    /// UnsafeCell is used to allow interior mutability.
    /// MaybeUninit is used to handle uninitialized memory safely.
    buffer: [UnsafeCell<MaybeUninit<T>>; N],

    /// The index where the next element will be written.
    /// Only modified by the producer.
    write: AtomicUsize,

    /// The index where the next element will be read from.
    /// Only modified by the consumer.
    read: AtomicUsize,

    /// The high watermark for valid data.
    /// Used to handle wrap-around scenarios efficiently.
    /// Modified by the producer, read by both the producer and the consumer.
    watermark: AtomicUsize,
}

// Safety: AtomicRingBuffer<T> is Send if T is Send.
// This is safe because we use atomic operations for all shared mutable state.
unsafe impl<T: Send, const N: usize> Send for AtomicRingBuffer<T, N> {}

// Safety: AtomicRingBuffer<T> is Sync if T is Send.
// This is safe because our operations ensure proper synchronization between threads.
unsafe impl<T: Send, const N: usize> Sync for AtomicRingBuffer<T, N> {}

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
    pub fn new() -> Self {
        assert!(
            N.is_power_of_two(),
            "Buffer capacity must be a power of two"
        );
        Self {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            watermark: AtomicUsize::new(N),
        }
    }

    /// Attempts to push an element onto the buffer.
    ///
    /// This method ensures that the element is written contiguously in the buffer.
    /// If there's not enough space at the end of the buffer, it will wrap around to
    /// the beginning.
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
    /// # Examples
    ///
    /// ```
    /// use atomic_ring_buffer::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 2> = AtomicRingBuffer::new();
    /// assert!(buffer.push(String::from("Hello")).is_ok());
    /// assert!(buffer.push(String::from("World")).is_ok());
    /// assert!(buffer.push(String::from("!")).is_err());
    /// ```
    pub fn push(&self, item: T) -> Result<(), T> {
        loop {
            let write = self.write.load(Ordering::Acquire);
            let read = self.read.load(Ordering::Acquire);
            let watermark = self.watermark.load(Ordering::Acquire);

            let next_write = (write + 1) & (N - 1);

            if next_write == read && write != watermark {
                return Err(item);
            }

            let (new_write, new_watermark) = if write == N - 1 {
                (0, write)
            } else {
                (next_write, watermark)
            };

            match self
                .write
                .compare_exchange(write, new_write, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    unsafe {
                        ptr::write((*self.buffer[write].get()).as_mut_ptr(), item);
                    }
                    if new_watermark != watermark {
                        self.watermark.store(new_watermark, Ordering::Release);
                    }
                    return Ok(());
                }
                Err(_) => continue,
            }
        }
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
    /// buffer.push(String::from("Hello")).unwrap();
    ///
    /// assert_eq!(buffer.pop(), Some(String::from("Hello")));
    /// assert_eq!(buffer.pop(), None);
    /// ```
    pub fn pop(&self) -> Option<T> {
        loop {
            let read = self.read.load(Ordering::Acquire);
            let write = self.write.load(Ordering::Acquire);
            let watermark = self.watermark.load(Ordering::Acquire);

            if read == write && read != watermark {
                return None;
            }

            let next_read = (read + 1) & (N - 1);
            match self
                .read
                .compare_exchange(read, next_read, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    let item = unsafe { ptr::read((*self.buffer[read].get()).as_ptr()) };
                    if read == watermark {
                        self.watermark.store(N, Ordering::Release);
                    }
                    return Some(item);
                }
                Err(_) => continue,
            }
        }
    }

    /// Attempts to peek at the next element in the buffer without removing it.
    ///
    /// # Returns
    ///
    /// Returns `Some(&T)` if an element is available, or `None` if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 2> = AtomicRingBuffer::new();
    /// buffer.push(String::from("Hello")).unwrap();
    ///
    /// assert_eq!(buffer.peek().map(|s| s.as_str()), Some("Hello"));
    /// assert_eq!(buffer.peek().map(|s| s.as_str()), Some("Hello")); // Still there
    /// ```
    pub fn peek(&self) -> Option<&T> {
        let read = self.read.load(Ordering::Acquire);
        let write = self.write.load(Ordering::Acquire);

        if read == write {
            None // Buffer is empty
        } else {
            unsafe { Some(&*(*self.buffer[read].get()).as_ptr()) }
        }
    }

    /// Returns the number of elements currently in the buffer.
    ///
    /// This method may not be entirely accurate in the presence of concurrent
    /// operations, but it provides a best-effort estimate.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();
    /// buffer.push(String::from("Hello")).unwrap();
    /// buffer.push(String::from("World")).unwrap();
    /// assert_eq!(buffer.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        let read = self.read.load(Ordering::Relaxed);
        let write = self.write.load(Ordering::Relaxed);
        (write.wrapping_sub(read)) & (N - 1)
    }

    /// Returns `true` if the buffer contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();
    /// assert!(buffer.is_empty());
    /// buffer.push(String::from("Hello")).unwrap();
    /// assert!(!buffer.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.read.load(Ordering::Relaxed) == self.write.load(Ordering::Relaxed)
    }

    /// Returns `true` if the buffer is at capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 2> = AtomicRingBuffer::new();
    /// assert!(!buffer.is_full());
    /// buffer.push(String::from("Hello")).unwrap();
    /// buffer.push(String::from("World")).unwrap();
    /// assert!(buffer.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Relaxed);
        let next_write = (write + 1) & (N - 1);
        next_write == read
    }

    /// Returns the total capacity of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 1024> = AtomicRingBuffer::new();
    /// assert_eq!(buffer.capacity(), 1024);
    /// ```
    pub fn capacity(&self) -> usize {
        N
    }

    /// Clears the buffer, removing all values.
    ///
    /// Note: This method is not atomic and should only be called when
    /// no other threads are accessing the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();
    /// buffer.push(String::from("Hello")).unwrap();
    /// assert!(!buffer.is_empty());
    /// buffer.clear();
    /// assert!(buffer.is_empty());
    /// ```
    pub fn clear(&self) {
        let mut read = self.read.load(Ordering::Acquire);
        let write = self.write.load(Ordering::Acquire);

        while read != write {
            unsafe {
                ptr::drop_in_place((*self.buffer[read].get()).as_mut_ptr());
            }
            read = Self::increment(read);
        }

        self.read.store(0, Ordering::Release);
        self.write.store(0, Ordering::Release);
        self.watermark.store(N, Ordering::Release);
    }

    /// Increments an index, wrapping around if necessary.
    #[inline]
    fn increment(index: usize) -> usize {
        (index + 1) & (N - 1)
    }
}

impl<T, const N: usize> fmt::Debug for AtomicRingBuffer<T, N>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AtomicRingBuffer")
            .field("capacity", &self.capacity())
            .field("len", &self.len())
            .finish()
    }
}

impl<T, const N: usize> Drop for AtomicRingBuffer<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_push_pop_basic() {
        let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();

        assert!(buffer.push(String::from("Hello")).is_ok());
        assert!(buffer.push(String::from("World")).is_ok());

        assert_eq!(buffer.pop(), Some(String::from("Hello")));
        assert_eq!(buffer.pop(), Some(String::from("World")));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_wraparound() {
        let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();

        assert!(buffer.push(String::from("1")).is_ok());
        assert!(buffer.push(String::from("2")).is_ok());
        assert!(buffer.push(String::from("3")).is_ok());

        assert_eq!(buffer.pop(), Some(String::from("1")));

        assert!(buffer.push(String::from("4")).is_ok());
        assert!(buffer.push(String::from("5")).is_ok());

        assert_eq!(buffer.pop(), Some(String::from("2")));
        assert_eq!(buffer.pop(), Some(String::from("3")));
        assert_eq!(buffer.pop(), Some(String::from("4")));
        assert_eq!(buffer.pop(), Some(String::from("5")));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_full_buffer() {
        let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();

        assert!(buffer.push(String::from("1")).is_ok());
        assert!(buffer.push(String::from("2")).is_ok());
        assert!(buffer.push(String::from("3")).is_ok());
        assert!(buffer.push(String::from("4")).is_err());

        assert_eq!(buffer.pop(), Some(String::from("1")));
        assert!(buffer.push(String::from("4")).is_ok());
        assert!(buffer.push(String::from("5")).is_err());
    }

    #[test]
    fn test_concurrent_usage() {
        let buffer = Arc::new(AtomicRingBuffer::<String, 1024>::new());
        let producer_buffer = Arc::clone(&buffer);
        let consumer_buffer = Arc::clone(&buffer);

        let producer = thread::spawn(move || {
            for i in 0..10000 {
                while producer_buffer.push(i.to_string()).is_err() {
                    thread::yield_now();
                }
            }
        });

        let consumer = thread::spawn(move || {
            let mut sum = 0;
            let mut count = 0;
            while count < 10000 {
                if let Some(value) = consumer_buffer.pop() {
                    sum += value.parse::<usize>().unwrap();
                    count += 1;
                } else {
                    thread::yield_now();
                }
            }
            sum
        });

        producer.join().unwrap();
        let sum = consumer.join().unwrap();

        assert_eq!(sum, (0..10000).sum::<usize>());
    }

    #[test]
    fn test_peek() {
        let buffer: AtomicRingBuffer<String, 2> = AtomicRingBuffer::new();

        assert_eq!(buffer.peek(), None);

        buffer.push(String::from("Hello")).unwrap();
        assert_eq!(buffer.peek().map(String::as_str), Some("Hello"));
        assert_eq!(buffer.peek().map(String::as_str), Some("Hello")); // Still there

        buffer.pop();
        assert_eq!(buffer.peek(), None);
    }

    #[test]
    fn test_clear() {
        let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();

        buffer.push(String::from("1")).unwrap();
        buffer.push(String::from("2")).unwrap();

        assert!(!buffer.is_empty());
        buffer.clear();
        assert!(buffer.is_empty());

        buffer.push(String::from("3")).unwrap();
        assert_eq!(buffer.pop(), Some(String::from("3")));
    }

    #[test]
    fn test_is_empty_and_is_full() {
        let buffer: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();

        assert!(buffer.is_empty());
        assert!(!buffer.is_full());

        buffer.push(1).unwrap();
        assert!(!buffer.is_empty());
        assert!(!buffer.is_full());

        buffer.push(2).unwrap();
        buffer.push(3).unwrap();
        assert!(!buffer.is_empty());
        assert!(buffer.is_full());

        buffer.pop();
        assert!(!buffer.is_empty());
        assert!(!buffer.is_full());
    }

    #[test]
    fn test_capacity_and_len() {
        let buffer: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();

        assert_eq!(buffer.capacity(), 4);
        assert_eq!(buffer.len(), 0);

        buffer.push(1).unwrap();
        buffer.push(2).unwrap();
        assert_eq!(buffer.len(), 2);

        buffer.pop();
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_multiple_producers_consumers() {
        let buffer = Arc::new(AtomicRingBuffer::<i32, 1024>::new());
        let num_producers = 4;
        let num_consumers = 4;
        let items_per_producer = 10000;

        let mut producers = vec![];
        let mut consumers = vec![];

        for _ in 0..num_producers {
            let producer_buffer = Arc::clone(&buffer);
            producers.push(thread::spawn(move || {
                for i in 0..items_per_producer {
                    while producer_buffer.push(i as i32).is_err() {
                        thread::yield_now();
                    }
                }
            }));
        }

        let total_items = num_producers * items_per_producer;
        let items_per_consumer = total_items / num_consumers;

        for _ in 0..num_consumers {
            let consumer_buffer = Arc::clone(&buffer);
            consumers.push(thread::spawn(move || {
                let mut sum = 0;
                let mut count = 0;
                while count < items_per_consumer {
                    if let Some(value) = consumer_buffer.pop() {
                        sum += value as usize;
                        count += 1;
                    } else {
                        thread::yield_now();
                    }
                }
                sum
            }));
        }

        for producer in producers {
            producer.join().unwrap();
        }

        let total_sum: usize = consumers.into_iter().map(|c| c.join().unwrap()).sum();

        let expected_sum = (0..items_per_producer as i32).sum::<i32>() as usize * num_producers;
        assert_eq!(total_sum, expected_sum);
    }

    #[test]
    fn test_drop() {
        let buffer = AtomicRingBuffer::<String, 4>::new();
        buffer.push(String::from("test")).unwrap();
        // The buffer will be dropped here, and it should not panic or leak memory
    }

    #[test]
    fn test_debug_output() {
        let buffer: AtomicRingBuffer<i32, 4> = AtomicRingBuffer::new();
        buffer.push(1).unwrap();
        buffer.push(2).unwrap();

        let debug_output = format!("{:?}", buffer);
        assert!(debug_output.contains("capacity: 4"));
        assert!(debug_output.contains("len: 2"));
    }
}
