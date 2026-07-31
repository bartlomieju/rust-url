use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_single_short");
    let short = "https://example.com/bench";
    g.throughput(Throughput::Bytes(short.len() as u64));
    g.bench_function("ada-3.4.6", |b| {
        b.iter(|| black_box(ada_url::Url::parse(black_box(short), None).unwrap()))
    });
    g.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
