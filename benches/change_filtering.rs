//! Benchmark for change filtering performance
//!
//! Compares the performance of iterator-based filtering (zero-copy)
//! vs clone-based filtering for large change sets.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rjd::{Change, Changes};
use serde_json::json;
use std::str::FromStr;

fn create_large_changes(count: usize) -> Changes {
    let mut changes = Changes::new();
    for i in 0..count {
        let path_str = format!("item{}", i);
        let path = rjd::json_path::JsonPath::from_str(&path_str).unwrap();
        changes.push(Change::Modified {
            path,
            old_value: json!(i),
            new_value: json!(i + 1),
        });
    }
    changes
}

fn filter_clone_based(changes: &Changes, patterns: &[String]) -> Changes {
    changes.filter_ignore_patterns(patterns)
}

fn filter_iterator_based<'a>(changes: &'a Changes, patterns: &'a [String]) -> Vec<&'a Change> {
    changes.iter_filtered_changes(patterns).collect()
}

fn bench_change_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("change_filtering");

    // Test different change counts
    for size in [100, 1_000, 10_000].iter() {
        // Create test data with 10% of changes filtered
        let changes = create_large_changes(*size);
        let filter_count = size / 10;
        let patterns: Vec<String> = (0..filter_count).map(|i| format!("/item{}", i)).collect();

        // Benchmark clone-based approach
        group.bench_with_input(
            BenchmarkId::new("clone_based", size),
            &(&changes, &patterns),
            |b, (changes, patterns)| {
                b.iter(|| {
                    filter_clone_based(
                        std::hint::black_box(changes),
                        std::hint::black_box(patterns),
                    )
                })
            },
        );

        // Benchmark iterator-based approach
        group.bench_with_input(
            BenchmarkId::new("iterator_based", size),
            &(&changes, &patterns),
            |b, (changes, patterns)| {
                b.iter(|| {
                    filter_iterator_based(
                        std::hint::black_box(changes),
                        std::hint::black_box(patterns),
                    )
                })
            },
        );
    }

    group.finish();
}

fn bench_filtering_no_patterns(c: &mut Criterion) {
    let changes = create_large_changes(10_000);
    let patterns: Vec<String> = vec![];

    c.bench_function("no_patterns_clone", |b| {
        b.iter(|| {
            filter_clone_based(
                std::hint::black_box(&changes),
                std::hint::black_box(&patterns),
            )
        })
    });

    c.bench_function("no_patterns_iterator", |b| {
        b.iter(|| {
            filter_iterator_based(
                std::hint::black_box(&changes),
                std::hint::black_box(&patterns),
            )
        })
    });
}

fn bench_filtering_heavy(c: &mut Criterion) {
    let changes = create_large_changes(10_000);
    // Filter out 90% of changes
    let patterns: Vec<String> = (0..9_000).map(|i| format!("/item{}", i)).collect();

    c.bench_function("heavy_filtering_clone", |b| {
        b.iter(|| {
            filter_clone_based(
                std::hint::black_box(&changes),
                std::hint::black_box(&patterns),
            )
        })
    });

    c.bench_function("heavy_filtering_iterator", |b| {
        b.iter(|| {
            filter_iterator_based(
                std::hint::black_box(&changes),
                std::hint::black_box(&patterns),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_change_filtering,
    bench_filtering_no_patterns,
    bench_filtering_heavy
);
criterion_main!(benches);
