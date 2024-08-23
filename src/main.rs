use bma_benchmark::benchmark;
use elevator_system::simulations::{
    concurrency_elevator_system::concurrency_elevator_system,
    elevator_system_error_handling::elevator_system_error_handling,
    scheduling_elevator_system::scheduling_elevator_system,
};
use std::hint::black_box;

pub fn main() {
    // concurrency_elevator_system();
    // benchmark!(1, { scheduling_elevator_system() });
    // scheduling_elevator_system();
    elevator_system_error_handling();
}
