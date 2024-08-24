use bma_benchmark::benchmark;
use core::panic;
use crossbeam_channel::unbounded;
use rand::Rng;
use scheduled_thread_pool::JobHandle;
use scheduled_thread_pool::ScheduledThreadPool;
use serde::{Deserialize, Serialize};
use std::hint::black_box;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::{
    cmp::Ordering,
    sync::{
        mpsc::{self, Receiver},
        MutexGuard,
    },
};
use std::{collections::VecDeque, time::Duration};
use threadpool::ThreadPool;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    ButtonPressed(ButtonPressed),
    ElevatorUnderMaintenance(String),
    Complete(bool),
}

#[derive(Debug, PartialEq)]
pub enum ElevatorEvent {
    Maintenance(String),
    ButtonPress(ButtonPressed),
    Complete,
    PowerOutage,
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

                match self.elevator_current_floor.cmp(&target_floor) {
                    Ordering::Less => {
                        println!(
                            "\tElevator {} move up and stopped at floor {}",
                            self.id, target_floor
                        );
                    }
                    Ordering::Greater => {
                        println!(
                            "\tElevator {} move down and stopped at floor {}",
                            self.id, target_floor
                        );
                    }
                    Ordering::Equal => {
                        println!("\tElevator {} stopped at floor {}", self.id, target_floor);
                    }
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
        if let Some(first_request) = request_queue.front() {
            let queue = request_queue
                .iter()
                .take(request_queue_count)
                .cloned()
                .collect::<Vec<_>>();

            let direction = match first_request.current_floor.cmp(&first_request.target_floor) {
                Ordering::Greater => Direction::Down,
                Ordering::Less => Direction::Up,
                Ordering::Equal => unreachable!("Current floor cannot be equal to target floor"),
            };

            self.move_elevator(queue, direction);
        } else {
            println!("****ERROR: handle request error, request queue is empty");
        }

        self.elevator_current_floor
    }
}

pub fn elevator_process_request(
    elevator_name: &str,
    elevator_current_floor: &Arc<Mutex<usize>>,
    button_press_queue: &Mutex<VecDeque<ButtonPressed>>,
    elevator_requests_queue: &Mutex<VecDeque<ButtonPressed>>,
    elevator_request_s: &mpsc::Sender<QueueStatus>,
    complete_receiving_buttons: &Arc<Mutex<bool>>,
    complete: &Arc<Mutex<bool>>,
) {
    let elevator = Elevator::new_elevator(
        elevator_name.to_string(),
        *elevator_current_floor.lock().unwrap(),
    );
    if let Some(request_queue) = elevator.process_requests(button_press_queue.lock().unwrap()) {
        let mut elevator_requests_queue = elevator_requests_queue.lock().unwrap();
        let request_queue_count = request_queue.len();
        elevator_requests_queue.extend(request_queue);

        elevator_request_s
            .send(QueueStatus::NewQueue(request_queue_count))
            .unwrap();
    } else {
        match *complete_receiving_buttons.lock().unwrap() {
            true => {
                if !(*complete.lock().unwrap()) {
                    elevator_request_s.send(QueueStatus::Done).unwrap();
                    *complete.lock().unwrap() = true;
                } else {
                    elevator_request_s.send(QueueStatus::Empty).unwrap()
                }
            }
            false => {
                // println!("Done message already sent!");
            }
        }
    }
}

pub fn elevator_handle_request(
    elevator_name: &str,
    elevator_request_r: &Receiver<QueueStatus>,
    elevator_requests_queue: &Mutex<VecDeque<ButtonPressed>>,
    current_floor: &Arc<Mutex<usize>>,
    elevator_finish_s: &crossbeam_channel::Sender<()>,
    handle: &JobHandle,
) -> bool {
    let mut elevator =
        Elevator::new_elevator(elevator_name.to_string(), *current_floor.lock().unwrap());
    if let Ok(queue_status) = elevator_request_r.recv() {
        match queue_status {
            QueueStatus::NewQueue(request_queue_count) => {
                let mut request_queue = elevator_requests_queue.lock().unwrap();
                println!(
                    "\tElevator {} handle request of person {:?}",
                    elevator.id,
                    request_queue
                        .iter()
                        .map(|r| r.person_id)
                        .collect::<Vec<_>>()
                );
                let elevator_current_floor =
                    elevator.handle_requests(&request_queue, request_queue_count);

                *request_queue = request_queue.split_off(request_queue_count);
                *current_floor.lock().unwrap() = elevator_current_floor;
            }
            QueueStatus::Empty => {}
            QueueStatus::Done => {
                handle.cancel();
                elevator_finish_s.send(()).unwrap();
                // elevator_finish_s.send(()).expect("All task are finish");
                return true;
            }
        }
    }

    false
}

pub fn elevator_system() {
    // Button pressed
    let mut button_presses = VecDeque::from(vec![
        ButtonPressed::new_request(1, 0, 5),
        ButtonPressed::new_request(2, 1, 4),
        ButtonPressed::new_request(3, 1, 3),
        ButtonPressed::new_request(4, 2, 6),
        ButtonPressed::new_request(5, 2, 0),
        ButtonPressed::new_request(6, 5, 3),
        ButtonPressed::new_request(7, 5, 2),
    ]);

    let button_press_queue = Arc::new(Mutex::new(VecDeque::new()));
    let complete_receiving_buttons = Arc::new(Mutex::new(false));
    let scheduled_thread_pool = ScheduledThreadPool::new(2);
    let elevator_under_maintenence = Arc::new(Mutex::new(String::new()));

    let (event_sender, event_receiver) = channel();
    let (elevator_1_request_s, elevator_1_request_r) = channel();
    let (elevator_2_request_s, elevator_2_request_r) = channel();
    let (elevator_1_finish_s, elevator_1_finish_r) = unbounded();
    let (elevator_2_finish_s, elevator_2_finish_r) = unbounded();
    let (power_outage_sender, power_outage_receiver) = unbounded();

    let pool = ThreadPool::new(6);

    {
        let event_sender = event_sender.clone();
        pool.execute(move || {
            let mut rng = rand::thread_rng();

            let random_time = rng.gen_range(10..20);

            thread::sleep(Duration::from_millis(random_time * 10));

            let mut rng = rand::thread_rng();

            let random_num = rng.gen_range(0..2);

            if random_num == 0 {
                // panic!("Power Outage Occur!")
                event_sender.send(ElevatorEvent::PowerOutage).unwrap();
            }
        });
    }

    {
        // Elevator Maintenance Event
        let event_sender = event_sender.clone();
        pool.execute(move || {
            let mut rng = rand::thread_rng();

            let random_num = rng.gen_range(1..3);

            let mut elevator_chosen = "";

            if random_num == 1 {
                elevator_chosen = "A"
            } else if random_num == 2 {
                elevator_chosen = "B"
            }

            // Randomly trigger the broken time for elevator
            let random_time = rng.gen_range(10..20);

            thread::sleep(Duration::from_millis(random_time * 10));

            // Send message to elevator controller when the elevator is broken
            event_sender
                .send(ElevatorEvent::Maintenance(elevator_chosen.to_string()))
                .unwrap();
        });
    }

    {
        // Button Press Event
        let event_sender = event_sender.clone();

        pool.execute(move || loop {
            if let Some(button_pressed) = button_presses.pop_front() {
                // Generate people arrive at random time
                let mut rng = rand::thread_rng();
                let people_arrival_time = rng.gen_range(1..8);
                thread::sleep(Duration::from_millis(people_arrival_time * 10));

                // Send message to elevator controller when receive button pressed
                event_sender
                    .send(ElevatorEvent::ButtonPress(button_pressed))
                    .unwrap();
            } else {
                event_sender.send(ElevatorEvent::Complete).unwrap();
                break;
            }
        })
    };

    // Elevator Controller
    {
        let elevator_under_maintenance = Arc::clone(&elevator_under_maintenence);
        let complete_receiving_buttons = Arc::clone(&complete_receiving_buttons);
        let button_press_queue = Arc::clone(&button_press_queue);

        pool.execute(move || loop {
            match event_receiver.recv() {
                Ok(event) => {
                    match event {
                        ElevatorEvent::Maintenance(elevator) => {

                            println!("*** Elevator Controller: ELEVATOR {} BROKEN !!! ***", elevator);
                            println!(
                                "*** Elevator Controller: Elevator {} entering maintenance mode. ***",
                                elevator
                            );

                            println!(
                                "*** ELEVATOR {} UNDER MAINTENANCE !!! ***: ",
                                elevator
                            );
                            *elevator_under_maintenance.lock().unwrap() = elevator;
                        }
                        ElevatorEvent::ButtonPress(button_pressed) => {
                            // Serialize the message
                            println!(
                                "Person {} press lift button at floor {} to floor {} *****",
                                button_pressed.person_id,
                                button_pressed.current_floor,
                                button_pressed.target_floor
                            );


                            button_press_queue.lock().unwrap().push_back(button_pressed);
                        }
                        ElevatorEvent::Complete => {
                            println!("Elevator Controller: No more people");
                            *complete_receiving_buttons.lock().unwrap() = true;
                            break;
                        }
                        ElevatorEvent::PowerOutage => {
                            power_outage_sender.send(ElevatorEvent::PowerOutage).unwrap();
                        },
                    }
                }
                Err(_) => println!("Nothing yet..."),
            }
        });
    }

    {
        // Elevator 1 process request: Get all the people that need to fetch (Periodic Task)
        let elevator_name = "A";
        let elevator_1_current_floor = Arc::new(Mutex::new(0));
        let button_press_queue = Arc::clone(&button_press_queue);
        let elevator_requests_queue = Arc::new(Mutex::new(VecDeque::new()));
        let complete_receiving_buttons = Arc::clone(&complete_receiving_buttons);

        let handle = {
            let elevator_requests_queue = Arc::clone(&elevator_requests_queue);
            let elevator_1_current_floor = Arc::clone(&elevator_1_current_floor);
            let complete = Arc::new(Mutex::new(false));
            let elevator_under_maintenence = Arc::clone(&elevator_under_maintenence);

            scheduled_thread_pool.execute_at_fixed_rate(
                Duration::from_millis(5),
                Duration::from_millis(5),
                move || {
                    if *elevator_under_maintenence.lock().unwrap() == elevator_name {
                        elevator_1_request_s.send(QueueStatus::Done).unwrap();
                        return;
                    }

                    elevator_process_request(
                        elevator_name,
                        &elevator_1_current_floor,
                        &button_press_queue,
                        &elevator_requests_queue,
                        &elevator_1_request_s,
                        &complete_receiving_buttons,
                        &complete,
                    );
                },
            )
        };

        // Elevator 1 handling request - Aperiodic Task (will only execute when it receive the message)
        {
            let elevator_requests_queue = Arc::clone(&elevator_requests_queue);
            let elevator_1_current_floor = Arc::clone(&elevator_1_current_floor);

            pool.execute(move || {
                loop {
                    if elevator_handle_request(
                        elevator_name,
                        &elevator_1_request_r,
                        &elevator_requests_queue,
                        &elevator_1_current_floor,
                        &elevator_1_finish_s,
                        &handle,
                    ) {
                        // QueueStatus is done.
                        break;
                    }
                }
                println!("Elevator {elevator_name} stopped.");
            });
        }
    }
    // Elevator 2 process requests: Get all people that need to fetch (Periodic Task)
    {
        let elevator_name = "B";
        let elevator_2_current_floor = Arc::new(Mutex::new(0));
        let complete_receiving_buttons = Arc::clone(&complete_receiving_buttons);
        let button_press_queue = Arc::clone(&button_press_queue);
        let elevator_requests_queue = Arc::new(Mutex::new(VecDeque::new()));

        let handle = {
            let elevator_requests_queue = Arc::clone(&elevator_requests_queue);
            let elevator_2_current_floor = Arc::clone(&elevator_2_current_floor);
            let complete = Arc::new(Mutex::new(false));
            let elevator_under_maintenence = Arc::clone(&elevator_under_maintenence);

            scheduled_thread_pool.execute_at_fixed_rate(
                Duration::from_millis(5),
                Duration::from_millis(5),
                move || {
                    if *elevator_under_maintenence.lock().unwrap() == elevator_name {
                        elevator_2_request_s.send(QueueStatus::Done).unwrap();
                        return;
                    }

                    elevator_process_request(
                        elevator_name,
                        &elevator_2_current_floor,
                        &button_press_queue,
                        &elevator_requests_queue,
                        &elevator_2_request_s,
                        &complete_receiving_buttons,
                        &complete,
                    );
                },
            )
        };
        // Elevator 2 handling request (Aperiodic Task)
        {
            let elevator_requests_queue = Arc::clone(&elevator_requests_queue);
            let elevator_2_current_floor = Arc::clone(&elevator_2_current_floor);

            pool.execute(move || {
                loop {
                    if elevator_handle_request(
                        elevator_name,
                        &elevator_2_request_r,
                        &elevator_requests_queue,
                        &elevator_2_current_floor,
                        &elevator_2_finish_s,
                        &handle,
                    ) {
                        // QueueStatus is done.
                        break;
                    }
                }
                println!("Elevator {elevator_name} stopped.");
            });
        }
    }

    loop {
        let power_outage_receiver = power_outage_receiver.clone();

        if let Ok(message) = power_outage_receiver.try_recv() {
            if message == ElevatorEvent::PowerOutage {
                panic!("!!!!!!!!!!!!!!! Alert: Power Outage Occur !!!!!!!!!!!!!!!!!");
            }
        }

        let elevator_finished = !elevator_1_finish_r.is_empty() && !elevator_2_finish_r.is_empty();

        if !(*complete_receiving_buttons.lock().unwrap()) {
            continue;
        } else if elevator_finished {
            break;
        }
    }
}

pub fn elevator_system_error_handling() {
    benchmark!(1, {
        // let system = std::thread::spawn(elevator_system);
        // match system.join() {
        //     Ok(_) => println!("Finished without panic."),
        //     Err(_) => println!("System panic somewhere..."),
        // }
        elevator_system();
    });
}
