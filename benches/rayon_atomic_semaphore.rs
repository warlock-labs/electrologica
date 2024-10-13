use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use electrologica::AtomicSemaphore;
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

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

fn bench_semaphore_with_rayon(c: &mut Criterion) {
    let mut group = c.benchmark_group("atomic_semaphore_rayon");
    group.measurement_time(Duration::from_secs(10));
    group.warm_up_time(Duration::from_secs(3));

    let total_operations = 1_000_000;
    let parallelism_levels = [1, 4, 8, 16, 32, 64, 128, 192];

    for &parallelism in &parallelism_levels {
        group.throughput(Throughput::Elements(total_operations as u64));
        group.bench_function(BenchmarkId::new("ops_per_second", parallelism), |b| {
            b.iter(|| {
                let sem = AtomicSemaphore::new(parallelism as u64);
                let counter = AtomicUsize::new(0);
                let max_concurrent = AtomicUsize::new(0);
                let operations_completed = AtomicUsize::new(0);
                let operations_attempted = AtomicUsize::new(0);

                let start = Instant::now();

                (0..total_operations).into_par_iter().for_each(|_| {
                    operations_attempted.fetch_add(1, Ordering::Relaxed);
                    if let Some(_guard) = sem.acquire_guard() {
                        let current = counter.fetch_add(1, Ordering::Relaxed);
                        max_concurrent.fetch_max(current + 1, Ordering::Relaxed);

                        // Simulate some work
                        std::hint::spin_loop();

                        counter.fetch_sub(1, Ordering::Relaxed);
                        operations_completed.fetch_add(1, Ordering::Relaxed);
                    }
                });

                let duration = start.elapsed();
                let total_completed = operations_completed.load(Ordering::Relaxed);

                // We return the operations per second
                total_completed as f64 / duration.as_secs_f64()
            })
        });
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .with_profiler(profiling::FlamegraphProfiler::new(100));
    targets = bench_semaphore_with_rayon
}

criterion_main!(benches);
