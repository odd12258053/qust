use criterion::{criterion_group, criterion_main, Criterion};
use mio::Token;
use qust::message::Reply;

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Reply");
    group.bench_function("message", |b| {
        b.iter(|| {
            Reply::error(Token(0)).message();
        })
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
