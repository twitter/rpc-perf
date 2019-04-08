#[macro_use]
extern crate criterion;

use bytes::BytesMut;
use codec::Decoder;
use codec::Memcache;
use criterion::Criterion;

fn encode_get_benchmark(c: &mut Criterion) {
    let codec = Memcache::new();
    let mut buf = BytesMut::new();
    c.bench_function("memcache encode get", move |b| {
        b.iter(|| codec.get(&mut buf, b"0"))
    });
}

fn encode_set_benchmark(c: &mut Criterion) {
    let codec = Memcache::new();
    let mut buf = BytesMut::new();
    c.bench_function("memcache encode set", move |b| {
        b.iter(|| codec.set(&mut buf, b"0", b"0", None, None))
    });
}

fn memcache_decode_benchmark(c: &mut Criterion, label: &str, msg: &[u8]) {
    let codec = Memcache::new();
    let mut buf = BytesMut::with_capacity(1024);
    buf.extend_from_slice(msg);
    let buf = buf.freeze();
    c.bench_function(label, move |b| b.iter(|| codec.decode(&buf)));
}

fn decode_ok_benchmark(c: &mut Criterion) {
    memcache_decode_benchmark(c, "memcache decode ok", b"OK\r\n");
}

fn decode_incomplete_benchmark(c: &mut Criterion) {
    memcache_decode_benchmark(
        c,
        "memcache decode incomplete",
        b"VALUE 0 0 0\r\nSOME DATA GOES HERE\r\n",
    );
}

fn decode_hit_benchmark(c: &mut Criterion) {
    memcache_decode_benchmark(
        c,
        "memcache decode hit",
        b"VALUE 0 0 8\r\nDEADBEEF\r\nEND\r\n",
    );
}

fn decode_miss_benchmark(c: &mut Criterion) {
    memcache_decode_benchmark(c, "memcache decode miss", b"NOT_FOUND\r\n");
}

criterion_group!(
    benches,
    decode_hit_benchmark,
    decode_incomplete_benchmark,
    decode_miss_benchmark,
    decode_ok_benchmark,
    encode_get_benchmark,
    encode_set_benchmark,
);
criterion_main!(benches);
