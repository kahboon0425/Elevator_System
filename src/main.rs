mod simulations;
use bma_benchmark::benchmark;
use std::hint::black_box;
// use simulations::concurrency_elevator_system::concurrency_elevator_system;
use simulations::scheduling_elevator_system::scheduling_elevator_system;
pub fn main() {
    // concurrency_elevator_system();
    benchmark!(1, { scheduling_elevator_system() });
    // scheduling_elevator_system();
}
