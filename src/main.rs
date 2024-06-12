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

impl Elevator {
    pub fn move_up() {
        //logic
    }

    pub fn move_down() {}
}

fn main() {
    // share resources
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let queue_clone_1 = queue.clone();
    let queue_clone_2 = queue.clone();
    let queue_clone_3 = queue.clone();

    let (button_pressed_s, button_pressed_r) = channel();

    let pool = ThreadPool::new(3);

    // thread for sending request
    pool.execute(move || {
        let button_pressed = [
            ButtonPressed {
                person_id: 1,
                current_floor: 0,
                target_floor: 5,
            },
            ButtonPressed {
                person_id: 2,
                current_floor: 2,
                target_floor: 4,
            },
            ButtonPressed {
                person_id: 3,
                current_floor: 1,
                target_floor: 4,
            },
            ButtonPressed {
                person_id: 4,
                current_floor: 2,
                target_floor: 0,
            },
            ButtonPressed {
                person_id: 5,
                current_floor: 3,
                target_floor: 5,
            },
            ButtonPressed {
                person_id: 6,
                current_floor: 5,
                target_floor: 2,
            },
        ];

        for sequence in button_pressed {
            button_pressed_s.send(sequence).unwrap();
            thread::sleep(Duration::from_millis(500));
        }
    });

    pool.execute(move || {
        while let Ok(sequence) = button_pressed_r.recv() {
            println!("Received: {:?}", sequence);
            let mut queue = queue_clone_1.lock().unwrap();
            queue.push_front(sequence);
        }
    });

    // elevator 1
    pool.execute(move || loop {
        let elevator_1 = Elevator { id: 1 };
        let mut queue = queue_clone_2.lock().unwrap();
        let current_request = queue.pop_front();

        match current_request {
            Some(request) => println!(
                "Elevator {} handling request from person {:?}",
                elevator_1.id, request.person_id
            ),
            None => (),
        }
    });

    // elevator 2
    pool.execute(|| {
        let elevator_2 = Elevator { id: 2 };
        println!("{}", elevator_2.id);
    });

    loop {
        thread::sleep(Duration::from_secs(10));
    }
}
