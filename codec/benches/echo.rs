#[macro_use]
extern crate criterion;

use bytes::BytesMut;
use codec::Decoder;
use codec::Echo;
use criterion::Criterion;

fn encode_echo_benchmark(c: &mut Criterion) {
    let codec = Echo::new();
    c.bench_function("echo encode", move |b| {
        b.iter(|| codec.echo(&mut BytesMut::new(), b"0"))
    });
}

fn echo_decode_benchmark(c: &mut Criterion, label: &str, msg: &[u8]) {
    let codec = Echo::new();
    let mut buf = BytesMut::with_capacity(1024);
    buf.extend_from_slice(msg);
    let buf = buf.freeze();
    c.bench_function(label, move |b| b.iter(|| codec.decode(&buf)));
}

fn decode_error_benchmark(c: &mut Criterion) {
    echo_decode_benchmark(c, "echo decode error", b"3421780262\r\n");
}

fn decode_incomplete_benchmark(c: &mut Criterion) {
    echo_decode_benchmark(c, "echo decode incomplete", b"");
}

fn decode_ok_benchmark(c: &mut Criterion) {
    echo_decode_benchmark(c, "echo decode ok", &[0, 1, 2, 8, 84, 137, 127, 13, 10]);
}

criterion_group!(
    benches,
    decode_error_benchmark,
    decode_incomplete_benchmark,
    decode_ok_benchmark,
    encode_echo_benchmark,
);
criterion_main!(benches);
