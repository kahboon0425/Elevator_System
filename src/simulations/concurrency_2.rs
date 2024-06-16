extern crate threadpool;
use bma_benchmark::benchmark;
use crossbeam_channel::unbounded;
use std::hint::black_box;
use std::{
    collections::VecDeque,
    sync::{mpsc::channel, Arc, Mutex},
    thread,
    time::Duration,
};
use threadpool::ThreadPool;

pub struct Elevator {
    pub id: String,
    pub elevator_current_floor: usize,
}

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
        }
    }

    pub fn check_request(
        &self,
        queue: &Arc<Mutex<VecDeque<ButtonPressed>>>,
    ) -> Option<VecDeque<ButtonPressed>> {
        let mut queue = queue.lock().unwrap();
        let mut request_queue = VecDeque::new();

        // Pop the first request from the queue
        if let Some(request) = queue.pop_front() {
            request_queue.push_back(request);

            // Check the elevator direction, -1(move down), 1(move up)
            let initial_direction = if request.target_floor < request.current_floor {
                -1
            } else {
                1
            };

            // Check if there are other requests heading in the same direction
            let mut i = 0;
            while i < queue.len() {
                if let Some(next_request) = queue.get(i) {
                    let next_request_direction =
                        if next_request.target_floor < next_request.current_floor {
                            -1
                        } else {
                            1
                        };

                    // Check if the next request is in the same direction
                    let is_same_direction = initial_direction == next_request_direction;

                    // Check if the request current floor is higher than current floor
                    let is_higher_than_current_floor = initial_direction == 1
                        && next_request.current_floor > request.current_floor;
                    let is_lower_than_current_floor = initial_direction == -1
                        && next_request.current_floor < request.current_floor;

                    if is_same_direction
                        && (is_higher_than_current_floor || is_lower_than_current_floor)
                    {
                        let new_request = queue.remove(i).unwrap();
                        request_queue.push_back(new_request);
                        continue;
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
            // Find the min floor any person wants to enter
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

            if let Some(floor) = floor {
                let floor = if floor.entered {
                    floor.target_floor
                } else {
                    floor.current_floor
                };

                println!("Elevator {} stopped at floor {}", self.id, floor);

                // Get all the people that want to enter the lift
                let enter_person = request_queue
                    .iter_mut()
                    .filter(|r| r.current_floor == floor);
                for person in enter_person {
                    person.entered = true;
                    println!(
                        "Person {} enters elevator {} at floor {}",
                        person.person_id, self.id, floor
                    );
                }

                self.elevator_current_floor = floor;

                // Get all the people that want to exit lift
                let exit_idx = request_queue
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.entered && r.target_floor == floor)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();

                for (i, idx) in exit_idx.iter().enumerate() {
                    println!(
                        "Person {} exits elevator {} at floor {}",
                        request_queue[idx - i].person_id,
                        self.id,
                        floor
                    );
                    request_queue.remove(idx - i);
                }

                self.elevator_current_floor = floor;
            } else {
                break;
            }
        }
    }

    pub fn handle_request(&mut self, request_queue: VecDeque<ButtonPressed>) -> usize {
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
            ButtonPressed::new_request(3, 1, 5),
            ButtonPressed::new_request(4, 2, 6),
            ButtonPressed::new_request(5, 2, 0),
            ButtonPressed::new_request(6, 1, 4),
            ButtonPressed::new_request(7, 2, 0),
            ButtonPressed::new_request(8, 3, 5),
            ButtonPressed::new_request(9, 5, 2),
            ButtonPressed::new_request(10, 5, 2),
        ];

        for sequence in button_pressed {
            println!(
                "\t Person {} press lift button at floor {} to floor {}",
                sequence.person_id, sequence.current_floor, sequence.target_floor
            );
            button_pressed_s.send(sequence).unwrap();
            thread::sleep(Duration::from_millis(3));
        }
    });

    // thread for receiving request
    let queue_clone_1 = Arc::clone(&queue);
    pool.execute(move || {
        while let Ok(sequence) = button_pressed_r.recv() {
            let mut queue = queue_clone_1.lock().unwrap();
            queue.push_back(sequence);
        }
        s.send(()).unwrap();
    });

    // elevator 1
    let queue_clone_2 = Arc::clone(&queue);
    let r1_clone = r.clone();
    let mut elevator_1_current_floor = 0;
    pool.execute(move || loop {
        thread::sleep(Duration::from_millis(10));
        let mut elevator_1 = Elevator::new_elevator("A".to_string(), elevator_1_current_floor);
        if let Some(requests) = elevator_1.check_request(&queue_clone_2) {
            println!("Request Queue Elevator A: {:?}", requests);
            let elevator_current_floor = elevator_1.handle_request(requests);
            elevator_1_current_floor = elevator_current_floor;
        }
        if queue_clone_2.lock().unwrap().is_empty() {
            if !r1_clone.is_empty() {
                break;
            }
        }
    });

    // elevator 2
    let queue_clone_3 = Arc::clone(&queue);
    let r2_clone = r.clone();
    let mut elevator_2_current_floor = 0;
    pool.execute(move || loop {
        thread::sleep(Duration::from_millis(10));
        let mut elevator_2 = Elevator::new_elevator("B".to_string(), elevator_2_current_floor);
        if let Some(requests) = elevator_2.check_request(&queue_clone_3) {
            println!("Request Queue Elevator B: {:?}", requests);
            let elevator_current_floor = elevator_2.handle_request(requests);
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

pub fn concurrency_2() {
    benchmark!(1, {
        elevator_system();
    });
}
