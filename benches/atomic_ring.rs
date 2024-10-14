use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use electrologica::AtomicRingBuffer;
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
    group.sample_size(10);

    let base_operations = 10_000_000;
    let buffer_sizes = [4096];
    let thread_configs = [
        (1, 1),   // 1 producer, 1 consumer
        (2, 2),   // 2 producers, 2 consumers
        (4, 4),   // 4 producers, 4 consumers
        (8, 8),   // 8 producers, 8 consumers
        (16, 16), // 16 producers, 16 consumers
        (1, 16),  // 1 producer, 16 consumers
        (16, 1),  // 16 producers, 1 consumer
    ];

    for &buffer_size in &buffer_sizes {
        for &(producer_count, consumer_count) in &thread_configs {
            let total_operations = base_operations;
            let id = BenchmarkId::new(
                format!(
                    "size_{}_p{}_c{}",
                    buffer_size, producer_count, consumer_count
                ),
                total_operations,
            );
            group.throughput(Throughput::Elements(total_operations));
            group.bench_with_input(id, &total_operations, |b, &total_ops| {
                b.iter(|| {
                    let buffer = Arc::new(AtomicRingBuffer::<u64, 4096>::new());
                    let producer_ops = total_ops / producer_count;
                    let consumer_ops = total_ops / consumer_count;

                    // Producer threads
                    let producers: Vec<_> = (0..producer_count)
                        .map(|_| {
                            let buf = Arc::clone(&buffer);
                            std::thread::spawn(move || {
                                let mut retries = 0u64;
                                let max_retries = producer_ops / 10;
                                for i in 0..producer_ops {
                                    while buf.try_push(i).is_err() && retries < max_retries {
                                        retries += 1;
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
                                let mut retries = 0u64;
                                let max_retries = producer_ops / 10;
                                let mut sum = 0;
                                for _ in 0..consumer_ops {
                                    while buf.try_pop().is_none() && retries < max_retries {
                                        retries += 1;
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
                    let _: u64 = consumers.into_iter().map(|c| c.join().unwrap()).sum();

                    //assert_eq!(total_consumed, total_ops);
                })
            });
        }
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .significance_level(0.01)
        .noise_threshold(0.05)
        .with_profiler(profiling::FlamegraphProfiler::new(100));
    targets = bench_ring_buffer
}

criterion_main!(benches);
