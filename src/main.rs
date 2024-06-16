mod simulations;
use simulations::concurrency_1::concurrency_1;
use simulations::concurrency_2::concurrency_2;
pub fn main() {
    concurrency_1();
    concurrency_2();
}
