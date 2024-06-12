extern crate threadpool;
use std::{
    collections::VecDeque,
    sync::{mpsc::channel, Arc, Mutex},
    thread,
    time::Duration,
};
use threadpool::ThreadPool;

pub struct Elevator {
    id: usize,
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
    pub fn new_elevator(elevator_id: usize) -> Self {
        Elevator { id: elevator_id }
    }

    pub fn handling_request(
        elevator_id: usize,
        queue: &Arc<Mutex<VecDeque<ButtonPressed>>>,
    ) -> Option<(usize, usize, usize)> {
        let mut queue = queue.lock().unwrap();

        // Pop the first request from the queue
        if let Some(request) = queue.pop_front() {
            println!(
                "Elevator {} handling request from person {:?}",
                elevator_id, request.person_id
            );
            Some((
                request.person_id,
                request.current_floor,
                request.target_floor,
            ))
        } else {
            // If no request is found, return None
            None
        }
    }
}

fn main() {
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
            button_pressed_s.send(sequence).unwrap();
            thread::sleep(Duration::from_millis(100));
        }
    });

    // thread for receiving request
    let queue_clone_1 = Arc::clone(&queue);
    pool.execute(move || {
        while let Ok(sequence) = button_pressed_r.recv() {
            println!("Received: {:?}", sequence);
            let mut queue = queue_clone_1.lock().unwrap();
            queue.push_front(sequence);
        }
    });

    // elevator 1
    let queue_clone_2 = Arc::clone(&queue);
    pool.execute(move || loop {
        let elevator_1 = Elevator::new_elevator(1);
        if let Some((person_id, current_floor, target_floor)) =
            Elevator::handling_request(elevator_1.id, &queue_clone_2)
        {}
    });

    // elevator 2
    let queue_clone_3 = Arc::clone(&queue);
    pool.execute(move || loop {
        let elevator_2 = Elevator::new_elevator(2);
        if let Some((person_id, current_floor, target_floor)) =
            Elevator::handling_request(elevator_2.id, &queue_clone_3)
        {}
    });

    loop {
        thread::sleep(Duration::from_secs(10));
    }
}
