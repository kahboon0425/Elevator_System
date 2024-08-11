use bma_benchmark::benchmark;
use elevator_system::simulations::scheduling_elevator_system::scheduling_elevator_system;
use std::hint::black_box;

pub fn main() {
    // concurrency_elevator_system();
    benchmark!(1, { scheduling_elevator_system() });
    // scheduling_elevator_system();
}
