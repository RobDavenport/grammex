use criterion::{criterion_group, criterion_main, Criterion};

fn matching_benchmark(c: &mut Criterion) {
    c.bench_function("placeholder", |b| {
        b.iter(|| {
            // TODO: benchmark pattern matching on realistic graphs
            42
        })
    });
}

criterion_group!(benches, matching_benchmark);
criterion_main!(benches);
