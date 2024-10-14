# Electrologica

[![License](https://img.shields.io/crates/l/electrologica)](https://choosealicense.com/licenses/apache-2.0/)
[![Crates.io](https://img.shields.io/crates/v/electrologica)](https://crates.io/crates/electrologica)
[![Docs](https://img.shields.io/crates/v/electrologica?color=blue&label=docs)](https://docs.rs/electrologica/)
[![Rust](https://github.com/warlock-labs/electrologica/workflows/Rust/badge.svg)](https://github.com/warlock-labs/electrologica/actions)
[![codecov](https://codecov.io/gh/warlock-labs/electrologica/branch/main/graph/badge.svg?token=XXXXXXXXXX)](https://codecov.io/gh/warlock-labs/electrologica)

Electrologica is a high-performance, concurrent data structures library for Rust, optimized for nanosecond-level
latencies and high-contention scenarios. It provides lock-free data structures and synchronization primitives designed
to excel in environments where every nanosecond counts. Please note this crate is EXPERIMENTAL and not yet suitable for
production use.

## Features

- **Lock-free data structures**: Efficiently manage concurrent access without traditional locks
- **Atomic operations**: Carefully designed with precise memory ordering for optimal performance
- **Nanosecond-level optimizations**: Tailored for ultra-low latency requirements
- **Flexible spin-wait primitives**: Customizable synchronization tools for specific use cases
- **Nested parallelism support**: Ideal for complex, multi-layered concurrent systems
- **Congestion management**: Built-in mechanisms to handle high-contention scenarios effectively

## Installation

Add Electrologica to your `Cargo.toml`:

```toml
[dependencies]
electrologica = "0.3.0"
```

## Usage

### AtomicSemaphore

The `AtomicSemaphore` provides a high-performance mechanism for controlling access to a limited number of resources:

```rust
use electrologica::AtomicSemaphore;

fn main() {
    let sem = AtomicSemaphore::new(5);
    assert!(sem.try_acquire());
    assert_eq!(sem.available_permits(), 4);
    sem.release();
    assert_eq!(sem.available_permits(), 5);
}
```

### AtomicRingBuffer

The `AtomicRingBuffer` offers a lock-free, single-producer single-consumer queue for efficient inter-thread communication:

```rust
use electrologica::AtomicRingBuffer;

fn main() {
    let buffer: AtomicRingBuffer<String, 4> = AtomicRingBuffer::new();

    // Producer operations
    buffer.try_push(String::from("Hello")).unwrap();
    buffer.try_push(String::from("World")).unwrap();

    // Consumer operations
    assert_eq!(buffer.try_pop(), Some(String::from("Hello")));
    assert_eq!(buffer.try_pop(), Some(String::from("World")));
    assert_eq!(buffer.try_pop(), None);
}
```

### Spin Module

The `spin` module provides configurable spin-wait primitives for custom synchronization needs:

```rust
use electrologica::spin::{spin_try, SpinConfig, SpinError};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

fn main() {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = Arc::clone(&flag);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
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
            ..SpinConfig::default()
        }
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}
```

## Architecture

Electrologica is built around three core components:

1. **AtomicSemaphore**: A high-performance semaphore implementation that uses atomic operations to manage concurrent
   access to shared resources efficiently.

2. **AtomicRingBuffer**: A lock-free, single-producer single-consumer ring buffer that provides a fast and efficient way
   to pass data between threads without the need for mutexes or locks.

3. **Spin Module**: A collection of configurable spin-wait primitives that allow for fine-tuned control over busy-wait
   loops, essential for scenarios where traditional blocking mechanisms are too slow.

Each of these components is meticulously designed with careful attention to memory ordering and optimized for extremely
low-latency scenarios. The library leverages Rust's powerful type system and ownership model to provide safe
abstractions over low-level, high-performance concurrent programming primitives.

## Performance

Electrologica is designed from the ground up for high performance in concurrent scenarios. Some key performance
characteristics include:

- Lock-free algorithms to minimize contention and avoid kernel-mode transitions
- Careful use of atomic operations and memory ordering to ensure correctness with minimal overhead
- Optimized data structures that minimize cache line bouncing and false sharing
- Spin-wait primitives that can be fine-tuned for specific hardware and workload characteristics

We are committed to continual performance improvements and welcome benchmarks and performance reports from the
community.

## Security

While Electrologica strives for high performance, we take security seriously. However, please note:

- This library uses unsafe Rust to achieve its performance goals.
- It is not recommended for production use without thorough review and testing.
- The library has not undergone a formal security audit.

We encourage users to carefully evaluate the security implications of using Electrologica in their projects. If you
discover any security-related issues, please report them responsibly by emailing security@warlock.xyz instead of using
the public issue tracker.

## API Documentation

For detailed API documentation, please refer to the [API docs on docs.rs](https://docs.rs/electrologica/).

## Minimum Supported Rust Version (MSRV)

Electrologica's MSRV is `1.80`. We strive to maintain compatibility with stable Rust releases and will clearly
communicate any changes to the MSRV in our release notes.

## Contributing

We welcome contributions to Electrologica! Whether it's bug reports, feature requests, documentation improvements, or
code contributions, your input is valued.

Before contributing, please:

1. Check the [issue tracker](https://github.com/warlock-labs/electrologica/issues) to see if your topic has already been
   discussed.
2. Read our [Contributing Guide](CONTRIBUTING.md) for details on our development process, coding standards, and more.
3. Review our [Code of Conduct](CODE_OF_CONDUCT.md) to understand our community standards.

## License

Electrologica is licensed under the Apache License, Version 2.0.

## Inspiration

Electrologica draws its name and inspiration from the pioneering work of the Dutch company Electrologica, which produced
the X1 computer in the 1950s. The X1, first delivered in 1958, was one of the first commercially successful
transistorized computers in Europe.

Key features of the X1 that inspire our work:

- **Modularity**: The X1 was designed with a modular architecture, allowing for flexible configurations.
- **Speed**: For its time, the X1 was remarkably fast, with a basic cycle time of 64 microseconds.
- **Innovative Design**: It incorporated novel features like interrupt handling and a hardware floating point unit.

Just as the original Electrologica pushed the boundaries of what was possible in computing during the 1950s, our
Electrologica library aims to push the boundaries of concurrent programming in Rust. We strive to honor this legacy of
innovation by:

1. Prioritizing modularity and flexibility in our API design
2. Continuously optimizing for speed and efficiency
3. Incorporating cutting-edge techniques in concurrent programming

While modern computing challenges are vastly different from those of the 1950s, we believe that the spirit of innovation
and the pursuit of performance that drove the original Electrologica company are just as relevant today. Our library is
a tribute to those early pioneers and a commitment to continuing their legacy of advancing the field of computing.

As we develop and evolve Electrologica, we keep in mind the remarkable progress made in just a few decades - from
computers that measured speed in microseconds to our current nanosecond-scale optimizations. It's a reminder of the
rapid pace of technological advancement and the exciting possibilities that lie ahead in the field of high-performance
computing.
