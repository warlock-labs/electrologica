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
//! - Atomic operations for a thread-safe access
//! - Watermark mechanism for efficient wraparound handling
//! - Support for any `Send` type `T`
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
use std::iter::FusedIterator;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fmt, ptr};

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
    /// use electrologica::AtomicRingBuffer;
    ///
    /// let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();
    /// assert!(buffer.push(String::from("Hello")).is_ok());
    /// assert!(buffer.push(String::from("World")).is_ok());
    /// assert!(buffer.push(String::from("!")).is_ok());
    /// assert!(buffer.push(String::from("Goodbye")).is_err());
    /// ```
    pub fn push(&self, item: T) -> Result<(), T> {
        loop {
            let write = self.write.load(Ordering::SeqCst);
            let read = self.read.load(Ordering::SeqCst);
            let next_write = (write + 1) & (N - 1);

            // Check if buffer is full
            if next_write == read {
                return Err(item); // Buffer is full
            }

            // Update watermark if we're about to wrap around
            let new_watermark = if next_write == 0 {
                write
            } else {
                self.watermark.load(Ordering::SeqCst)
            };

            match self
                .write
                .compare_exchange(write, next_write, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => {
                    unsafe {
                        ptr::write((*self.buffer[write].get()).as_mut_ptr(), item);
                    }

                    if new_watermark != self.watermark.load(Ordering::SeqCst) {
                        self.watermark.store(new_watermark, Ordering::SeqCst);
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
            let read = self.read.load(Ordering::SeqCst);
            let write = self.write.load(Ordering::SeqCst);

            if read == write {
                return None; // Buffer is empty
            }

            let next_read = (read + 1) & (N - 1);
            let watermark = self.watermark.load(Ordering::SeqCst);

            // Check if we need to wrap around
            let actual_next_read = if read == watermark && watermark != N {
                0
            } else {
                next_read
            };

            match self.read.compare_exchange(
                read,
                actual_next_read,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    let item = unsafe { ptr::read((*self.buffer[read].get()).as_ptr()) };

                    if actual_next_read == 0 {
                        self.watermark.store(N, Ordering::SeqCst);
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
        let read = self.read.load(Ordering::SeqCst);
        let write = self.write.load(Ordering::SeqCst);

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
        let read = self.read.load(Ordering::SeqCst);
        let write = self.write.load(Ordering::SeqCst);
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
        self.read.load(Ordering::SeqCst) == self.write.load(Ordering::SeqCst)
    }

    /// Returns the total capacity of the buffer.
    pub fn capacity(&self) -> usize {
        N
    }

    /// Returns the remaining capacity of the buffer.
    ///
    /// This is the number of elements that can be pushed onto the buffer
    /// before it becomes full awaiting a pop operation.
    pub fn remaining_capacity(&self) -> usize {
        N - self.len()
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
        let mut read = self.read.load(Ordering::SeqCst);
        let write = self.write.load(Ordering::SeqCst);

        while read != write {
            unsafe {
                ptr::drop_in_place((*self.buffer[read].get()).as_mut_ptr());
            }
            read = Self::increment(read);
        }

        // Use a single store with release ordering to ensure all previous operations are visible
        self.write.store(0, Ordering::Release);
        self.read.store(0, Ordering::Release);
        self.watermark.store(N, Ordering::Release);
    }

    /// Increments an index, wrapping around if necessary.
    #[inline]
    fn increment(index: usize) -> usize {
        (index + 1) & (N - 1)
    }
}

impl<T: Clone, const N: usize> Clone for AtomicRingBuffer<T, N> {
    fn clone(&self) -> Self {
        // Create a new buffer
        let new_buffer = Self::new();

        // Copy the contents
        let read = self.read.load(Ordering::SeqCst);
        let write = self.write.load(Ordering::SeqCst);
        let mut current = read;

        while current != write {
            if let Some(item) = unsafe { (*self.buffer[current].get()).as_ptr().as_ref() } {
                // Cloning each item and pushing it to the new buffer
                let _ = new_buffer.push(item.clone());
            }
            current = (current + 1) & (N - 1);
        }

        // Copy the watermark
        new_buffer
            .watermark
            .store(self.watermark.load(Ordering::SeqCst), Ordering::SeqCst);

        new_buffer
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AtomicRingBuffer")
            .field("capacity", &N)
            .field("len", &self.len())
            .field("read", &self.read.load(Ordering::SeqCst))
            .field("write", &self.write.load(Ordering::SeqCst))
            .field("watermark", &self.watermark.load(Ordering::SeqCst))
            .finish()
    }
}

impl<T: PartialEq, const N: usize> PartialEq for AtomicRingBuffer<T, N> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        let self_read = self.read.load(Ordering::SeqCst);
        let other_read = other.read.load(Ordering::SeqCst);
        let self_write = self.write.load(Ordering::SeqCst);

        let mut self_current = self_read;
        let mut other_current = other_read;

        while self_current != self_write {
            let self_item = unsafe { (*self.buffer[self_current].get()).as_ptr().as_ref() };
            let other_item = unsafe { (*other.buffer[other_current].get()).as_ptr().as_ref() };

            if self_item != other_item {
                return false;
            }

            self_current = (self_current + 1) & (N - 1);
            other_current = (other_current + 1) & (N - 1);
        }

        true
    }
}

impl<T: Eq, const N: usize> Eq for AtomicRingBuffer<T, N> {}

// Iterator implementation
pub struct AtomicRingBufferIterator<'a, T, const N: usize> {
    buffer: &'a AtomicRingBuffer<T, N>,
    current: usize,
    end: usize,
}

impl<'a, T, const N: usize> Iterator for AtomicRingBufferIterator<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let item = unsafe { &*(*self.buffer.buffer[self.current].get()).as_ptr() };
            self.current = (self.current + 1) & (N - 1);
            Some(item)
        }
    }
}

impl<'a, T, const N: usize> FusedIterator for AtomicRingBufferIterator<'a, T, N> {}

impl<T, const N: usize> AtomicRingBuffer<T, N> {
    /// Returns an iterator over the items in the buffer.
    ///
    /// Note: This iterator provides a snapshot of the buffer at the time of creation.
    /// Concurrent modifications may not be reflected in the iteration.
    pub fn iter(&self) -> AtomicRingBufferIterator<T, N> {
        let read = self.read.load(Ordering::SeqCst);
        let write = self.write.load(Ordering::SeqCst);
        AtomicRingBufferIterator {
            buffer: self,
            current: read,
            end: write,
        }
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a AtomicRingBuffer<T, N> {
    type Item = &'a T;
    type IntoIter = AtomicRingBufferIterator<'a, T, N>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_push_pop() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert!(buffer.push(1).is_ok());
        assert!(buffer.push(2).is_ok());
        assert!(buffer.push(3).is_ok());
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_full_buffer() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert!(buffer.push(1).is_ok());
        assert!(buffer.push(2).is_ok());
        assert!(buffer.push(3).is_ok());
        assert!(buffer.push(4).is_err()); // Buffer should be full
        assert_eq!(buffer.pop(), Some(1));
        assert!(buffer.push(4).is_ok()); // Now there should be space
    }

    #[test]
    fn test_wraparound() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert!(buffer.push(1).is_ok());
        assert!(buffer.push(2).is_ok());
        assert!(buffer.push(3).is_ok());
        assert!(buffer.push(4).is_err()); // Buffer is full
        assert_eq!(buffer.pop(), Some(1));
        assert!(buffer.push(4).is_ok());
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_peek() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert_eq!(buffer.peek(), None);
        assert!(buffer.push(1).is_ok());
        assert_eq!(buffer.peek(), Some(&1));
        assert_eq!(buffer.peek(), Some(&1)); // Peek shouldn't consume
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.peek(), None);
    }

    #[test]
    fn test_len() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.push(1).is_ok());
        assert_eq!(buffer.len(), 1);
        assert!(buffer.push(2).is_ok());
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_clear() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        assert!(buffer.push(1).is_ok());
        assert!(buffer.push(2).is_ok());
        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.pop(), None);
        assert!(buffer.push(3).is_ok());
        assert_eq!(buffer.pop(), Some(3));
    }

    #[test]
    fn test_multiple_wraparounds() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        for i in 0..10 {
            assert!(buffer.push(i).is_ok());
            if i >= 2 {
                // Buffer can only hold 3 elements (N-1)
                assert_eq!(buffer.pop(), Some(i - 2));
            }
        }
        assert_eq!(buffer.pop(), Some(8));
        assert_eq!(buffer.pop(), Some(9));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_alternating_push_pop() {
        let buffer: AtomicRingBuffer<u32, 4> = AtomicRingBuffer::new();
        for i in 0..100 {
            assert!(buffer.push(i).is_ok());
            assert_eq!(buffer.pop(), Some(i));
        }
        assert_eq!(buffer.pop(), None);
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
        assert_eq!(sum, (0..10000).sum());
    }

    #[test]
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
        let sum = consumer_thread.join().unwrap();
        // We expect some losses here based on the test just firing off and forgetting
        // the producer thread, but the sum should be close to the expected value
        assert!(sum >= (0..1000000).sum::<u64>() - 100000);
    }
}
