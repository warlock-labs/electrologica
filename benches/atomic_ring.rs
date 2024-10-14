use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use electrologica::AtomicRingBuffer;
use rayon::prelude::*;
use std::sync::Arc;
use std::time::Duration;

mod profiling {
    use criterion::profiler::Profiler;
    use pprof::ProfilerGuard;
    use std::fs::File;
    use std::path::Path;

    pub struct FlamegraphProfiler<'a> {
        frequency: i32,
        active_profiler: Option<ProfilerGuard<'a>>,
    }

    impl<'a> FlamegraphProfiler<'a> {
        pub fn new(frequency: i32) -> Self {
            FlamegraphProfiler {
                frequency,
                active_profiler: None,
            }
        }
    }

    impl<'a> Profiler for FlamegraphProfiler<'a> {
        fn start_profiling(&mut self, _benchmark_id: &str, _benchmark_dir: &Path) {
            self.active_profiler = Some(ProfilerGuard::new(self.frequency).unwrap());
        }

        fn stop_profiling(&mut self, _benchmark_id: &str, benchmark_dir: &Path) {
            std::fs::create_dir_all(benchmark_dir).unwrap();
            let flamegraph_path = benchmark_dir.join("flamegraph.svg");
            let flamegraph_file = File::create(&flamegraph_path)
                .expect("File system error while creating flamegraph.svg");

            if let Some(profiler) = self.active_profiler.take() {
                profiler
                    .report()
                    .build()
                    .unwrap()
                    .flamegraph(flamegraph_file)
                    .expect("Error writing flamegraph");
            }
        }
    }
}

fn bench_ring_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("atomic_ring_buffer");
    group.measurement_time(Duration::from_secs(10));
    group.warm_up_time(Duration::from_secs(3));

    let total_operations = 1_000_000;
    let buffer_sizes = [64, 256, 1024, 4096];
    let parallelism_levels = [1, 4, 8, 16, 32];

    for &buffer_size in &buffer_sizes {
        for &parallelism in &parallelism_levels {
            group.throughput(Throughput::Elements(total_operations as u64));
            group.bench_function(
                BenchmarkId::new(format!("size_{}", buffer_size), parallelism),
                |b| {
                    b.iter(|| {
                        let buffer = Arc::new(AtomicRingBuffer::<u64, 4096>::new());
                        let producer_count = parallelism / 2;
                        let consumer_count = parallelism - producer_count;

                        let producer_ops = total_operations / producer_count;
                        let consumer_ops = total_operations / consumer_count;

                        // Producer threads
                        let producers: Vec<_> = (0..producer_count)
                            .map(|_| {
                                let buf = Arc::clone(&buffer);
                                std::thread::spawn(move || {
                                    for i in 0..producer_ops {
                                        while buf.push(i as u64).is_err() {
                                            std::hint::spin_loop();
                                        }
                                    }
                                })
                            })
                            .collect();

                        // Consumer threads
                        let consumers: Vec<_> = (0..consumer_count)
                            .map(|_| {
                                let buf = Arc::clone(&buffer);
                                std::thread::spawn(move || {
                                    let mut sum = 0;
                                    for _ in 0..consumer_ops {
                                        while let None = buf.pop() {
                                            std::hint::spin_loop();
                                        }
                                        sum += 1;
                                    }
                                    sum
                                })
                            })
                            .collect();

                        // Join threads
                        for producer in producers {
                            producer.join().unwrap();
                        }
                        let total_consumed: u64 = consumers.into_iter().map(|c| c.join().unwrap()).sum();

                        assert_eq!(total_consumed, total_operations as u64);
                    })
                },
            );
        }
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .with_profiler(profiling::FlamegraphProfiler::new(100));
    targets = bench_ring_buffer
}

criterion_main!(benches);