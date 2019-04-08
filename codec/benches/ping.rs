#[macro_use]
extern crate criterion;

use codec::Decoder;
use bytes::BytesMut;
use criterion::Criterion;
use codec::Ping;

fn encode_ping_benchmark(c: &mut Criterion) {
    let codec = Ping::new();
    let mut buf = BytesMut::new();
    c.bench_function("ping encode", move |b| b.iter(|| codec.ping(&mut buf)));
}

fn ping_decode_benchmark(c: &mut Criterion, label: &str, msg: &[u8]) {
    let codec = Ping::new();
    let mut buf = BytesMut::with_capacity(1024);
    buf.extend_from_slice(msg);
    let buf = buf.freeze();
    c.bench_function(label, move |b| b.iter(|| codec.decode(&buf)));
}

fn decode_ok_benchmark(c: &mut Criterion) {
    ping_decode_benchmark(c, "ping decode ok", b"+PONG\r\n");
}

fn decode_incomplete_benchmark(c: &mut Criterion) {
    ping_decode_benchmark(c, "ping decode incomplete", b"+PONG");
}

fn decode_unknown_benchmark(c: &mut Criterion) {
    ping_decode_benchmark(c, "ping decode unknown", b"+PONG\r\nDEADBEEF\r\n",);
}

criterion_group!(
    benches,
    decode_incomplete_benchmark,
    decode_ok_benchmark,
    decode_unknown_benchmark,
    encode_ping_benchmark,
);
criterion_main!(benches);
