use atomic_arena::Arena;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn reserve_slots(size: usize) {
	let arena = Arena::<()>::new(size);
	let controller = arena.controller();
	for _ in 0..size {
		controller.try_reserve().unwrap();
	}
}

fn benchmark(c: &mut Criterion) {
	let sizes = [100, 10_000];
	for size in sizes {
		c.bench_with_input(
			BenchmarkId::new("reserve slots", size),
			&size,
			|b, num_slots| {
				b.iter(|| reserve_slots(*num_slots));
			},
		);
	}
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
