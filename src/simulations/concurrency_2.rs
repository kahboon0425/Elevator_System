extern crate threadpool;
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

#[derive(Debug)]
pub struct ButtonPressed {
    person_id: usize,
    current_floor: usize,
    target_floor: usize,
}

impl ButtonPressed {
    pub fn new_request(p_id: usize, c_floor: usize, t_floor: usize) -> Self {
        ButtonPressed {
            person_id: p_id,
            current_floor: c_floor,
            target_floor: t_floor,
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
        elevator_id: &String,
        queue: &Arc<Mutex<VecDeque<ButtonPressed>>>,
    ) -> Option<VecDeque<(usize, usize, usize)>> {
        let mut queue = queue.lock().unwrap();
        let mut request_queue = VecDeque::new();

        // if !queue.is_empty() {
        //     println!("Requests: {:?}", queue);
        // }

        // Pop the first request from the queue
        if let Some(request) = queue.pop_front() {
            println!(
                "\tElevator {} handling request from person {:?} #####",
                elevator_id, request.person_id
            );

            request_queue.push_back((
                request.person_id,
                request.current_floor,
                request.target_floor,
            ));

            // Check the elevator direction, -1(move down), 1(move up)
            let initial_direction = if request.target_floor < request.current_floor {
                -1
            } else {
                1
            };

            let mut final_floor = request.target_floor;

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
                        if initial_direction == 1 && next_request.target_floor > final_floor {
                            final_floor = next_request.target_floor;
                        } else if initial_direction == -1 && next_request.target_floor < final_floor
                        {
                            final_floor = next_request.target_floor;
                        }

                        let new_request = queue.remove(i).unwrap();
                        request_queue.push_back((
                            new_request.person_id,
                            new_request.current_floor,
                            new_request.target_floor,
                        ));
                        continue;
                    }
                }
                i += 1;
            }

            // Check if there are requests starting from the final floor
            let mut j = 0;
            while j < queue.len() {
                if let Some(next_request) = queue.get(j) {
                    if next_request.current_floor == final_floor {
                        let new_request = queue.remove(j).unwrap();
                        request_queue.push_back((
                            new_request.person_id,
                            new_request.current_floor,
                            new_request.target_floor,
                        ));
                        continue;
                    }
                }
                j += 1;
            }

            Some(request_queue)
        } else {
            // If no request is found, return None
            None
        }
    }

    pub fn move_up(
        &mut self,
        first_request: (usize, usize, usize),
        mut request_queue: VecDeque<(usize, usize, usize)>,
    ) {
        while self.elevator_current_floor < first_request.1 {
            self.elevator_current_floor += 1;
            println!(
                "\tElevator {} at floor {}",
                self.id, self.elevator_current_floor
            );
        }
        let mut target_floor = first_request.2;

        let mut new_queue = Vec::new();

        println!("\tPerson {} enters elevator {}", first_request.0, self.id);
        while !request_queue.is_empty() {
            let min_current_floor = request_queue.iter().min_by_key(|r| &r.1).unwrap();
            // println!("Min current floor: {:?}", min_current_floor);
            let index = request_queue
                .iter()
                .position(|r| r == min_current_floor)
                .unwrap();
            let remove_queue = request_queue.remove(index).unwrap();
            new_queue.push(remove_queue);

            while self.elevator_current_floor < remove_queue.1 {
                self.elevator_current_floor += 1;
                println!(
                    "Elevator {} at floor {}",
                    self.id, self.elevator_current_floor
                );
                println!("Person {} enters elevator {}", remove_queue.0, self.id);
            }
        }

        while !new_queue.is_empty() {
            let min_target_floor = new_queue.iter().min_by_key(|t| &t.2).unwrap();
            println!("Min target floor {:?}", min_target_floor);
            let index = new_queue
                .iter()
                .position(|r| r == min_target_floor)
                .unwrap();
            println!("Index of min element {}", index);
            let remove_queue = new_queue.remove(index);
            println!("Remove New Queue: {:?}", remove_queue);
        }
    }

    pub fn move_down(
        &mut self,
        first_request: (usize, usize, usize),
        mut request_queue: VecDeque<(usize, usize, usize)>,
    ) {
        let mut target_floor = first_request.2;
        println!("\tPerson {} enters elevator {}", first_request.0, self.id);
        while !request_queue.is_empty() {
            let (person_id, user_current_floor, user_target_floor) =
                request_queue.pop_front().unwrap();
            if user_target_floor < target_floor {
                target_floor = user_target_floor;
            }

            while self.elevator_current_floor > first_request.1 {
                self.elevator_current_floor -= 1;
                println!(
                    "\tElevator {} at floor {}",
                    self.id, self.elevator_current_floor
                );
                if self.elevator_current_floor == user_current_floor {
                    println!("\tPerson {} enters elevator {}", person_id, self.id);
                }
            }

            while self.elevator_current_floor > first_request.2 {
                self.elevator_current_floor -= 1;
                println!(
                    "\tElevator {} at floor {}",
                    self.id, self.elevator_current_floor
                );
                if self.elevator_current_floor == user_target_floor {
                    println!("\tPerson {} exits elevator {}", person_id, self.id);
                }
            }
        }
    }

    pub fn open_lift_door(&self) {
        println!(
            "\tElevator {} opening door at floor {}",
            self.id, self.elevator_current_floor
        );
    }

    pub fn close_lift_door(&self) {
        println!(
            "\tElevator {} closing door at floor {}",
            self.id, self.elevator_current_floor
        );
    }

    pub fn handle_request(
        &mut self,
        mut request_queue: VecDeque<(usize, usize, usize)>, // person_id: usize,
                                                            // user_current_floor: usize,
                                                            // user_target_floor: usize,
    ) -> usize {
        let first_request = request_queue.pop_front().unwrap();
        if first_request.1 > first_request.2 {
            // self.move_down(first_request, request_queue);
        } else if first_request.1 < first_request.2 {
            self.move_up(first_request, request_queue);
        }

        // self.move_down(first_request, &request_queue);

        self.elevator_current_floor
    }
}

pub fn concurrency_2() {
    // share resources
    let queue = Arc::new(Mutex::new(VecDeque::new()));

    // sender, receiver
    let (button_pressed_s, button_pressed_r) = channel();
    // let (queue_s, queue_r) = channel();

    let pool = ThreadPool::new(4);

    // thread for sending request
    pool.execute(move || {
        let button_pressed = [
            ButtonPressed::new_request(1, 0, 5),
            ButtonPressed::new_request(2, 2, 4),
            ButtonPressed::new_request(3, 1, 4),
            ButtonPressed::new_request(4, 2, 0),
            ButtonPressed::new_request(5, 3, 5),
            ButtonPressed::new_request(6, 5, 2),
        ];

        for sequence in button_pressed {
            println!(
                "Person {} press lift button at floor {} to floor {}",
                sequence.person_id, sequence.current_floor, sequence.target_floor
            );
            button_pressed_s.send(sequence).unwrap();
            // thread::sleep(Duration::from_millis(10));
        }
    });

    // thread for receiving request
    let queue_clone_1 = Arc::clone(&queue);
    pool.execute(move || {
        while let Ok(sequence) = button_pressed_r.recv() {
            println!("Received: {:?}", sequence);
            let mut queue = queue_clone_1.lock().unwrap();
            queue.push_back(sequence);
        }
    });

    // elevator 1
    let queue_clone_2 = Arc::clone(&queue);
    let mut elevator_1_current_floor = 0;
    pool.execute(move || loop {
        thread::sleep(Duration::from_millis(10));
        let mut elevator_1 = Elevator::new_elevator("A".to_string(), elevator_1_current_floor);
        if let Some(requests) = elevator_1.check_request(&elevator_1.id, &queue_clone_2) {
            println!("Requests Elevators 1: {:?}", requests);

            let elevator_current_floor = elevator_1.handle_request(requests);
            // elevator_1_current_floor = elevator_current_floor;
        }
    });

    // elevator 2
    let queue_clone_3 = Arc::clone(&queue);
    let mut elevator_2_current_floor = 0;
    pool.execute(move || loop {
        thread::sleep(Duration::from_millis(10));
        let mut elevator_2 = Elevator::new_elevator("B".to_string(), elevator_2_current_floor);
        if let Some(requests) = elevator_2.check_request(&elevator_2.id, &queue_clone_3) {
            println!("Requests Elevators 2: {:?}", requests);

            // let elevator_current_floor = elevator_2.handle_request(requests);
            // elevator_2_current_floor = elevator_current_floor;
        }
    });

    loop {
        thread::sleep(Duration::from_secs(100));
    }
}
