use criterion::{criterion_group, criterion_main, Criterion};
use elevator_system::simulations::concurrency_elevator_system::concurrency_elevator_system;
use elevator_system::simulations::scheduling_elevator_system::scheduling_elevator_system;

fn elevator_system_concurrency(c: &mut Criterion) {
    // concurrency_elevator_system();
    c.bench_function("Scheduling Elevator System", |b| {
        b.iter(|| concurrency_elevator_system());
    });
}
fn elevator_system_scheduling(c: &mut Criterion) {
    // scheduling_elevator_system();
    c.bench_function("Scheduling Elevator System", |b| {
        b.iter(|| scheduling_elevator_system());
    });
}

criterion_group!(
    benches,
    elevator_system_concurrency,
    elevator_system_scheduling
);
criterion_main!(benches);
