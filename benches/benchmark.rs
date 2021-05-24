use atomic_arena::Arena;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

fn benchmark(c: &mut Criterion) {
	let sizes = [100, 10_000];
	for size in sizes {
		c.bench_with_input(BenchmarkId::new("reserve slots", size), &size, |b, size| {
			b.iter_batched(
				|| Arena::<()>::new(*size).controller(),
				|controller| {
					for _ in 0..*size {
						controller.try_reserve().unwrap();
					}
				},
				BatchSize::SmallInput,
			);
		});
		c.bench_with_input(BenchmarkId::new("insert", size), &size, |b, size| {
			b.iter_batched(
				|| Arena::new(*size),
				|mut arena| {
					for i in 0..*size {
						arena.insert(i).unwrap();
					}
				},
				BatchSize::SmallInput,
			);
		});
	}
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
