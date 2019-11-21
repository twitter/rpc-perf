// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use metrics_core::*;

use criterion::{criterion_group, criterion_main, Bencher, Criterion};

use std::sync::atomic::{AtomicU64, Ordering};

fn thread_id(bencher: &mut Bencher) {
    bencher.iter(|| std::thread::current().id())
}

struct AtomicCounter {
    ctr: AtomicU64,
}

impl AtomicCounter {
    pub fn new() -> Self {
        Self {
            ctr: AtomicU64::new(0),
        }
    }
}

impl MetricCommon for AtomicCounter {}

impl Counter for AtomicCounter {
    fn store(&self, _: Instant, val: u64) {
        self.ctr.store(val, Ordering::Relaxed);
    }

    fn load(&self) -> u64 {
        self.ctr.load(Ordering::Relaxed)
    }

    fn add(&self, _: Instant, amount: u64) {
        self.ctr.fetch_add(amount, Ordering::Relaxed);
    }
}

struct Noop;

impl MetricCommon for Noop {}
impl Counter for Noop {
    fn store(&self, _: Instant, _: u64) {}
    fn load(&self) -> u64 {
        0
    }
    fn add(&self, _: Instant, _: u64) {}
}

fn increment_counter(bench: &mut Bencher) {
    let counter = AtomicCounter::new();
    let _scoped =
        unsafe { ScopedMetric::counter("test.metric", &counter, Metadata::empty()).unwrap() };

    bench.iter(|| {
        increment!("test.metric", MetricValue::Unsigned(10));
    })
}

fn set_noop_metric(bench: &mut Bencher) {
    let counter = Noop;
    let _scoped =
        unsafe { ScopedMetric::counter("test.noop", &counter, Metadata::empty()).unwrap() };

    bench.iter(|| {
        value!("test.noop", MetricValue::Unsigned(56));
    })
}

fn noop_metric_external_counter(bench: &mut Bencher) {
    let counter = Noop;
    let _scoped = unsafe {
        ScopedMetric::counter("test.noop.external-time", &counter, Metadata::empty()).unwrap()
    };

    let time = Instant::now();

    bench.iter(|| {
        value!(
            "test.noop.external-time",
            MetricValue::Unsigned(56),
            time = time
        )
    })
}

fn atomic_add(b: &mut Bencher) {
    let ctr = AtomicU64::new(0);

    b.iter(|| ctr.fetch_add(37, Ordering::Relaxed));
}

fn mutex_lock(b: &mut Bencher) {
    let mutex = std::sync::Mutex::new(());

    b.iter(|| mutex.lock());
}

fn current_time(b: &mut Bencher) {
    b.iter(|| Instant::now());
}

fn std_current_time(b: &mut Bencher) {
    b.iter(|| std::time::Instant::now());
}

fn bench_all(b: &mut Criterion) {
    b.bench_function("mutex_lock", mutex_lock);
    b.bench_function("atomic_add", atomic_add);
    b.bench_function("thread_id", thread_id);
    b.bench_function("current_time", current_time);
    b.bench_function("std_current_time", std_current_time);
    b.bench_function("set_noop_metric", set_noop_metric);
    b.bench_function("increment_counter", increment_counter);
    b.bench_function("noop_metric_external_counter", noop_metric_external_counter);
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_all
);

criterion_main!(benches);
