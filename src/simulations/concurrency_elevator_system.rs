extern crate threadpool;
use bma_benchmark::benchmark;
use crossbeam_channel::unbounded;
use peak_alloc::PeakAlloc;
use std::hint::black_box;
use std::{
    collections::VecDeque,
    sync::{mpsc::channel, Arc, Mutex},
    thread,
    time::Duration,
};
use threadpool::ThreadPool;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

pub struct Elevator {
    pub id: String,
    pub elevator_current_floor: usize,
    pub capacity: usize,
}

#[derive(PartialEq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ButtonPressed {
    person_id: usize,
    current_floor: usize,
    target_floor: usize,
    entered: bool,
}

impl ButtonPressed {
    pub fn new_request(p_id: usize, c_floor: usize, t_floor: usize) -> Self {
        ButtonPressed {
            person_id: p_id,
            current_floor: c_floor,
            target_floor: t_floor,
            entered: false,
        }
    }
}

impl Elevator {
    pub fn new_elevator(elevator_id: String, current_floor: usize) -> Self {
        Elevator {
            id: elevator_id,
            elevator_current_floor: current_floor,
            capacity: 5,
        }
    }

    pub fn process_requests(
        &self,
        queue: &Arc<Mutex<VecDeque<ButtonPressed>>>,
    ) -> Option<VecDeque<ButtonPressed>> {
        let mut queue = queue.lock().unwrap();
        let mut request_queue = VecDeque::new();

        // Pop the first request from the queue
        if let Some(request) = queue.pop_front() {
            request_queue.push_back(request);

            let initial_direction = if request.target_floor < request.current_floor {
                Direction::Down
            } else {
                Direction::Up
            };

            // Check if there are other requests heading in the same direction
            let mut i = 0;
            while i < queue.len() {
                if let Some(next_request) = queue.get(i) {
                    let next_request_direction =
                        if next_request.target_floor < next_request.current_floor {
                            Direction::Down
                        } else {
                            Direction::Up
                        };

                    // Check if the next request is in the same direction
                    let is_same_direction = initial_direction == next_request_direction;

                    let is_in_direction = match initial_direction {
                        Direction::Up => next_request.current_floor >= request.current_floor,
                        Direction::Down => next_request.current_floor <= request.current_floor,
                    };

                    if is_same_direction && is_in_direction {
                        if request_queue.len() == self.capacity {
                            break;
                        } else {
                            let new_request = queue.remove(i).unwrap();
                            request_queue.push_back(new_request);
                            continue;
                        }
                    }
                }
                i += 1;
            }

            Some(request_queue)
        } else {
            // If no request is found, return None
            None
        }
    }

    pub fn move_elevator(
        &mut self,
        mut request_queue: VecDeque<ButtonPressed>,
        direction: Direction,
    ) {
        loop {
            let floor = match direction {
                Direction::Up => request_queue.iter().min_by_key(|r| {
                    if r.entered {
                        r.target_floor
                    } else {
                        r.current_floor
                    }
                }),
                Direction::Down => request_queue.iter().max_by_key(|r| {
                    if r.entered {
                        r.target_floor
                    } else {
                        r.current_floor
                    }
                }),
            };

            if let Some(&request) = floor {
                let target_floor = if request.entered {
                    request.target_floor
                } else {
                    request.current_floor
                };

                if self.elevator_current_floor < target_floor {
                    println!(
                        "\tElevator {} move up and stopped at floor {}",
                        self.id, target_floor
                    );
                } else if self.elevator_current_floor > target_floor {
                    println!(
                        "\tElevator {} move down and stopped at floor {}",
                        self.id, target_floor
                    );
                } else {
                    println!("\tElevator {} stopped at floor {}", self.id, target_floor);
                }

                // Get all the people that want to enter the lift
                request_queue
                    .iter_mut()
                    .filter(|r| r.current_floor == target_floor)
                    .for_each(|r| {
                        r.entered = true;
                        println!(
                            "Person {} enters elevator {} at floor {}",
                            r.person_id, self.id, target_floor
                        );
                    });

                self.elevator_current_floor = target_floor;

                // Get all the people that want to exit lift
                let exit_idx = request_queue
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.entered && r.target_floor == target_floor)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();

                for (i, idx) in exit_idx.iter().enumerate() {
                    println!(
                        "Person {} exits elevator {} at floor {}",
                        request_queue[idx - i].person_id,
                        self.id,
                        target_floor
                    );
                    request_queue.remove(idx - i);
                }

                self.elevator_current_floor = target_floor;
            } else {
                break;
            }
        }
    }

    pub fn handle_requests(&mut self, request_queue: VecDeque<ButtonPressed>) -> usize {
        let first_request = request_queue.get(0).unwrap();
        if first_request.current_floor > first_request.target_floor {
            self.move_elevator(request_queue, Direction::Down);
        } else if first_request.current_floor < first_request.target_floor {
            self.move_elevator(request_queue, Direction::Up);
        }

        self.elevator_current_floor
    }
}

pub fn elevator_system() {
    // share resources
    let queue = Arc::new(Mutex::new(VecDeque::new()));

    // sender, receiver
    let (button_pressed_s, button_pressed_r) = channel();
    let (s, r) = unbounded();

    let pool = ThreadPool::new(4);

    // thread for sending request
    pool.execute(move || {
        let button_pressed = [
            ButtonPressed::new_request(1, 0, 5),
            ButtonPressed::new_request(2, 1, 4),
            ButtonPressed::new_request(3, 1, 3),
            ButtonPressed::new_request(4, 2, 6),
            ButtonPressed::new_request(5, 2, 0),
            ButtonPressed::new_request(6, 5, 3),
            ButtonPressed::new_request(7, 5, 2),
        ];

        for sequence in button_pressed {
            button_pressed_s.send(sequence).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
    });

    // thread for receiving request
    let queue_clone_1 = Arc::clone(&queue);
    pool.execute(move || {
        while let Ok(sequence) = button_pressed_r.recv() {
            let mut queue = queue_clone_1.lock().unwrap();
            queue.push_back(sequence);
            println!(
                "Person {} press lift button at floor {} to floor {} *****",
                sequence.person_id, sequence.current_floor, sequence.target_floor
            );
        }
        // Tell when finished
        s.send(()).unwrap();
    });

    // elevator 1
    let queue_clone_2 = queue.clone();
    let r1_clone = r.clone();
    let mut elevator_1_current_floor = 0;
    pool.execute(move || loop {
        let mut elevator_1 = Elevator::new_elevator("A".to_string(), elevator_1_current_floor);
        thread::sleep(Duration::from_millis(5));
        if let Some(requests) = elevator_1.process_requests(&queue_clone_2) {
            println!(
                "\tElevator {} handle request of person {:?}",
                elevator_1.id,
                requests.iter().map(|r| r.person_id).collect::<Vec<_>>()
            );
            let elevator_current_floor = elevator_1.handle_requests(requests);
            elevator_1_current_floor = elevator_current_floor;
        }
        if queue_clone_2.lock().unwrap().is_empty() {
            if !r1_clone.is_empty() {
                break;
            }
        }
    });

    // elevator 2
    let queue_clone_3 = queue.clone();
    let r2_clone = r.clone();
    let mut elevator_2_current_floor = 0;
    pool.execute(move || loop {
        let mut elevator_2 = Elevator::new_elevator("B".to_string(), elevator_2_current_floor);
        thread::sleep(Duration::from_millis(5));
        if let Some(requests) = elevator_2.process_requests(&queue_clone_3) {
            println!(
                "\tElevator {} handle request of person {:?}",
                elevator_2.id,
                requests.iter().map(|r| r.person_id).collect::<Vec<_>>()
            );
            let elevator_current_floor = elevator_2.handle_requests(requests);
            elevator_2_current_floor = elevator_current_floor;
        }

        if queue_clone_3.lock().unwrap().is_empty() {
            if !r2_clone.is_empty() {
                break;
            }
        }
    });

    pool.join();
}

pub fn concurrency_elevator_system() {
    benchmark!(1, {
        elevator_system();
    });
    let current_mem = PEAK_ALLOC.current_usage_as_kb();
    println!("\nThis program currently uses {} KB of RAM.", current_mem);
    let peak_mem = PEAK_ALLOC.peak_usage_as_kb();
    println!("The max amount that was used {}", peak_mem);
}
