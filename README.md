# Electrologica

[![Crates.io](https://img.shields.io/crates/v/electrologica.svg)](https://crates.io/crates/electrologica)
[![Documentation](https://docs.rs/electrologica/badge.svg)](https://docs.rs/electrologica)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Electrologica is a high-performance concurrent data structures crate for Rust, designed for scenarios where
nanosecond-level latencies matter and contention in highly parallel systems is a concern.

## Features

- **AtomicSemaphore**: An extremely low-latency semaphore optimized for high-contention scenarios, particularly useful
  in nested parallelism contexts.
- Designed to manage congestion in parallel data structure updates. 
- Optimized for scenarios involving frequent updates to shared data structures.

## Use Cases

Electrologica is particularly well-suited for:

- Nested parallelism scenarios where fine-grained concurrency control is needed
- Systems requiring congestion control in highly parallel data structure updates

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
electrologica = "0.1.0"
```

## Usage

### AtomicSemaphore

```rust
use electrologica::AtomicSemaphore;

let sem = AtomicSemaphore::new(5);

// Acquire a permit
if sem.acquire() {
// Do some work
// ...
// Release the permit
sem.release();
}

// Use RAII guard
if let Some(guard) = sem.acquire_guard() {
// Do some work
// Permit is automatically released when guard goes out of scope
}
```

## Performance

The `AtomicSemaphore` is designed to provide extremely low latency in high-contention scenarios. It uses a combination
of atomic operations and optimized spinning strategies to minimize overhead and maximize throughput.

Benchmark results and comparisons with other synchronization primitives are available in the `benches/` directory.

## Contributing

We welcome contributions to Electrologica! Please feel free to submit issues, fork the repository and send pull
requests.

To contribute:

1. Fork the repository (https://github.com/warlock-labs/electrologica/fork)
2. Create your feature branch (`git checkout -b my-new-feature`)
3. Commit your changes (`git commit -am 'Add some feature'`)
4. Push to the branch (`git push origin my-new-feature`)
5. Create a new Pull Request

## License

This project is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

Electrologica is inspired by the pioneering work in computer engineering by the Electrologica company, which produced
the X1 computer in the 1950s. We aim to honor their legacy by pushing the boundaries of what's possible in modern,
high-performance computing.

The development of this library has been influenced by research in network congestion control algorithms and optimized
graph traversal techniques.