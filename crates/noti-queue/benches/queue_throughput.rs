//! Queue throughput benchmarks for noti-queue.
//!
//! Run with: cargo bench --bench queue_throughput
//!
//! These benchmarks measure:
//! - Enqueue throughput (tasks/second)
//! - Dequeue throughput (tasks/second)
//! - Enqueue + dequeue roundtrip throughput
//! - Concurrent enqueue throughput
//! - Concurrent worker throughput (enqueue + dequeue + ack)

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use noti_core::{Message, Priority, ProviderConfig};
use noti_queue::{InMemoryQueue, NotificationTask, QueueBackend};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Shared Tokio runtime to avoid per-iteration setup cost.
///
/// Criterion's `b.iter()` is called thousands of times. Creating a new
/// `Runtime` inside `b.iter()` adds ~100µs of overhead per sample, which
/// inflates wall-time and masks the actual queue operation cost.  Instead we
/// create one runtime per benchmark function (outside `b.iter()`) and drive
/// all async operations through it via `rt.block_on(...)`.
fn make_runtime() -> Runtime {
    Runtime::new().expect("create tokio runtime")
}

/// Create a minimal NotificationTask for benchmarking.
fn make_task(provider: &str, priority: Priority) -> NotificationTask {
    let msg = Message::text("benchmark task").with_priority(priority);
    NotificationTask::new(provider, ProviderConfig::new(), msg)
}

fn bench_enqueue(c: &mut Criterion) {
    let rt = make_runtime();
    let queue = Arc::new(InMemoryQueue::new());

    let mut group = c.benchmark_group("enqueue");
    group.throughput(Throughput::Elements(1));

    for size in [1, 100, 1000, 10000] {
        // Pre-populate queue to the correct size outside b.iter()
        rt.block_on(async {
            for _ in 0..size {
                queue
                    .enqueue(make_task("slack", Priority::Normal))
                    .await
                    .unwrap();
            }
        });

        group.bench_function(format!("{}_tasks", size), |b| {
            b.iter(|| {
                rt.block_on(async {
                    for _ in 0..size {
                        let task = make_task("slack", Priority::Normal);
                        black_box(queue.enqueue(task).await).unwrap();
                    }
                });
            });
        });
    }
    group.finish();
}

fn bench_dequeue(c: &mut Criterion) {
    let rt = make_runtime();
    let queue = Arc::new(InMemoryQueue::new());

    // Pre-fill queue outside b.iter()
    rt.block_on(async {
        for _ in 0..10000 {
            queue
                .enqueue(make_task("slack", Priority::Normal))
                .await
                .unwrap();
        }
    });

    let mut group = c.benchmark_group("dequeue");
    group.throughput(Throughput::Elements(1));

    for size in [1, 100, 1000, 10000] {
        group.bench_function(format!("{}_tasks", size), |b| {
            b.iter(|| {
                rt.block_on(async {
                    for _ in 0..size {
                        black_box(queue.dequeue().await).unwrap();
                    }
                });
            });
        });
    }
    group.finish();
}

fn bench_enqueue_dequeue_roundtrip(c: &mut Criterion) {
    let rt = make_runtime();

    let mut group = c.benchmark_group("enqueue_dequeue_roundtrip");
    group.throughput(Throughput::Elements(1));

    for size in [1, 100, 1000] {
        group.bench_function(format!("{}_tasks", size), |b| {
            b.iter(|| {
                rt.block_on(async {
                    let queue = InMemoryQueue::new();
                    for _ in 0..size {
                        let task = make_task("slack", Priority::Normal);
                        queue.enqueue(task).await.unwrap();
                        black_box(queue.dequeue().await).unwrap();
                    }
                });
            });
        });
    }
    group.finish();
}

fn bench_concurrent_enqueue(c: &mut Criterion) {
    let rt = make_runtime();

    let mut group = c.benchmark_group("concurrent_enqueue");
    group.throughput(Throughput::Elements(1));

    for concurrency in [1, 4, 8, 16] {
        let tasks_per_thread = 1000;
        let total = concurrency * tasks_per_thread;

        group.bench_function(
            format!("{}_threads_{}_each", concurrency, tasks_per_thread),
            |b| {
                b.iter(|| {
                    rt.block_on(async {
                        let queue = Arc::new(InMemoryQueue::new());
                        let mut handles = vec![];

                        for _ in 0..concurrency {
                            let queue = Arc::clone(&queue);
                            let handle = tokio::spawn(async move {
                                for _ in 0..tasks_per_thread {
                                    let task = make_task("slack", Priority::Normal);
                                    black_box(queue.enqueue(task).await).unwrap();
                                }
                            });
                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }

                        // Verify total
                        let stats = queue.stats().await.unwrap();
                        assert_eq!(stats.queued, total);
                    });
                });
            },
        );
    }
    group.finish();
}

fn bench_ack_throughput(c: &mut Criterion) {
    let rt = make_runtime();

    let mut group = c.benchmark_group("ack_throughput");
    group.throughput(Throughput::Elements(1));

    for size in [1, 100, 1000] {
        group.bench_function(format!("{}_tasks", size), |b| {
            b.iter(|| {
                rt.block_on(async {
                    let queue = InMemoryQueue::new();
                    let mut task_ids = vec![];

                    // Enqueue and dequeue to get processing tasks
                    for _ in 0..size {
                        let task = make_task("slack", Priority::Normal);
                        let id = queue.enqueue(task).await.unwrap();
                        let _dequeued = queue.dequeue().await.unwrap().unwrap();
                        task_ids.push(id);
                    }

                    // ACK all
                    for id in &task_ids {
                        black_box(queue.ack(id).await).unwrap();
                    }
                });
            });
        });
    }
    group.finish();
}

fn bench_priority_ordering(c: &mut Criterion) {
    // Runtime created once, outside b.iter()
    let rt = make_runtime();
    let queue = Arc::new(InMemoryQueue::new());

    // Pre-populate with 1000 tasks outside b.iter()
    rt.block_on(async {
        let priorities = [
            Priority::Low,
            Priority::Normal,
            Priority::High,
            Priority::Urgent,
        ];
        for pri in priorities.iter().cycle().take(1000) {
            queue.enqueue(make_task("slack", *pri)).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("priority_ordering");

    group.bench_function("enqueue_4_priorities", |b| {
        b.iter(|| {
            rt.block_on(async {
                let queue = InMemoryQueue::new();
                let priorities = [
                    Priority::Low,
                    Priority::Normal,
                    Priority::High,
                    Priority::Urgent,
                ];
                for pri in priorities.iter().cycle().take(1000) {
                    let task = make_task("slack", *pri);
                    black_box(queue.enqueue(task).await).unwrap();
                }
            });
        });
    });

    group.bench_function("dequeue_priority_order", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..250 {
                    black_box(queue.dequeue().await).unwrap();
                }
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_enqueue,
    bench_dequeue,
    bench_enqueue_dequeue_roundtrip,
    bench_concurrent_enqueue,
    bench_ack_throughput,
    bench_priority_ordering
);
criterion_main!(benches);
