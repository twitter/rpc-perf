#[macro_use]
extern crate criterion;

use bytes::BytesMut;
use codec::Decoder;
use codec::{Redis, RedisMode};
use criterion::Criterion;

fn encode_inline_get_benchmark(c: &mut Criterion) {
    let codec = Redis::new(RedisMode::Inline);
    let mut buf = BytesMut::new();
    c.bench_function("redis inline encode get", move |b| {
        b.iter(|| codec.get(&mut buf, b"0"))
    });
}

fn encode_inline_set_benchmark(c: &mut Criterion) {
    let codec = Redis::new(RedisMode::Inline);
    let mut buf = BytesMut::new();
    c.bench_function("redis inline encode set", move |b| {
        b.iter(|| codec.set(&mut buf, b"0", b"0", None))
    });
}

fn encode_resp_get_benchmark(c: &mut Criterion) {
    let codec = Redis::new(RedisMode::Resp);
    let mut buf = BytesMut::new();
    c.bench_function("redis resp encode get", move |b| {
        b.iter(|| codec.get(&mut buf, b"0"))
    });
}

fn encode_resp_set_benchmark(c: &mut Criterion) {
    let codec = Redis::new(RedisMode::Resp);
    let mut buf = BytesMut::new();
    c.bench_function("redis resp encode set", move |b| {
        b.iter(|| codec.set(&mut buf, b"0", b"0", None))
    });
}

fn redis_decode_benchmark(c: &mut Criterion, label: &str, msg: &[u8]) {
    let codec = Redis::new(RedisMode::Inline);
    let mut buf = BytesMut::with_capacity(1024);
    buf.extend_from_slice(msg);
    let buf = buf.freeze();
    c.bench_function(label, move |b| b.iter(|| codec.decode(&buf)));
}

fn decode_ok_benchmark(c: &mut Criterion) {
    redis_decode_benchmark(c, "redis decode ok", b"+OK\r\n");
}

fn decode_incomplete_benchmark(c: &mut Criterion) {
    redis_decode_benchmark(c, "redis decode incomplete", b"$7\r\nHELLO\r\n");
}

fn decode_hit_benchmark(c: &mut Criterion) {
    redis_decode_benchmark(c, "redis decode hit", b"$8\r\nDEADBEEF\r\n");
}

fn decode_miss_benchmark(c: &mut Criterion) {
    redis_decode_benchmark(c, "redis decode miss", b"$-1\r\n");
}

criterion_group!(
    benches,
    decode_hit_benchmark,
    decode_incomplete_benchmark,
    decode_miss_benchmark,
    decode_ok_benchmark,
    encode_inline_get_benchmark,
    encode_inline_set_benchmark,
    encode_resp_get_benchmark,
    encode_resp_set_benchmark,
);
criterion_main!(benches);
