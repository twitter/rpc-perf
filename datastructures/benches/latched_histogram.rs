#[macro_use]
extern crate criterion;

use criterion::Criterion;
use datastructures::*;

fn latched_histogram_increment(c: &mut Criterion) {
    let histogram = LatchedHistogram::<u64>::new(60_000_000_000, 3);
    c.bench_function("latched histogram increment", move |b| {
        b.iter(|| histogram.increment(1_000_000_000, 1))
    });
}

fn latched_histogram_percentile(c: &mut Criterion) {
    let histogram = LatchedHistogram::<u64>::new(60_000_000_000, 3);
    histogram.increment(1_000_000_000, 1);
    c.bench_function("latched histogram percentile", move |b| {
        b.iter(|| histogram.percentile(1.0))
    });
}

criterion_group!(
    benches,
    latched_histogram_increment,
    latched_histogram_percentile,
);
criterion_main!(benches);
