use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::MutexGuard};

pub mod simulations;
// use simulations::concurrency_elevator_system::concurrency_elevator_system;
// use simulations::scheduling_elevator_system::scheduling_elevator_system;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    ButtonPressed(ButtonPressed),
    Complete(bool),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum QueueStatus {
    NewQueue(usize),
    Empty,
    Done,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    pub queue_status: QueueStatus,
    // elevator_requests_queue: VecDeque<ButtonPressed>,
    pub elevator_current_floor: usize,
}

pub struct Elevator {
    pub id: String,
    pub elevator_current_floor: usize,
    pub capacity: usize,
    pub status: String,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct ButtonPressed {
    pub person_id: usize,
    pub current_floor: usize,
    pub target_floor: usize,
    pub entered: bool,
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

#[derive(PartialEq)]
pub enum Direction {
    Up,
    Down,
}

impl Elevator {
    pub fn new_elevator(elevator_id: String, current_floor: usize) -> Self {
        Elevator {
            id: elevator_id,
            elevator_current_floor: current_floor,
            capacity: 5,
            status: "Idle".to_string(),
        }
    }

    /// Find all users to fetch.
    pub fn process_requests(
        &self,
        mut queue: MutexGuard<VecDeque<ButtonPressed>>,
    ) -> Option<VecDeque<ButtonPressed>> {
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

    pub fn move_elevator(&mut self, mut request_queue: Vec<ButtonPressed>, direction: Direction) {
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

    pub fn handle_requests(
        &mut self,
        request_queue: &VecDeque<ButtonPressed>,
        request_queue_count: usize,
    ) -> usize {
        if let Some(first_request) = request_queue.get(0) {
            let queue = request_queue
                .iter()
                .take(request_queue_count)
                .cloned()
                .collect::<Vec<_>>();
            let direction = if first_request.current_floor > first_request.target_floor {
                Direction::Down
            } else if first_request.current_floor < first_request.target_floor {
                Direction::Up
            } else {
                unreachable!("Current floor cannot be equal to target floor");
            };

            self.move_elevator(queue, direction);
        } else {
            println!("****ERROR: handle request error, request queue is empty");
        }

        self.elevator_current_floor
    }
}
