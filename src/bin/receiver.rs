use std::{
    collections::VecDeque,
    sync::{mpsc::channel, Arc, Mutex},
    time::Duration,
};

use amiquip::{Connection, ConsumerMessage, ConsumerOptions, QueueDeclareOptions, Result};
use bma_benchmark::benchmark;
use crossbeam_channel::unbounded;
use elevator_system::{elevator_handle_request, elevator_process_request, ButtonPressed, Message};
use peak_alloc::PeakAlloc;
use scheduled_thread_pool::ScheduledThreadPool;
use std::hint::black_box;
use threadpool::ThreadPool;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

pub fn elevator_system() {
    let button_press_queue = Arc::new(Mutex::new(VecDeque::new()));
    let complete_receiving_buttons = Arc::new(Mutex::new(false));
    let scheduled_thread_pool = ScheduledThreadPool::new(2);
    let pool = ThreadPool::new(2);
    let elevator_under_maintenence = Arc::new(Mutex::new(String::new()));

    let (elevator_1_request_s, elevator_1_request_r) = channel();
    let (elevator_2_request_s, elevator_2_request_r) = channel();
    let (elevator_1_finish_s, elevator_1_finish_r) = unbounded();
    let (elevator_2_finish_s, elevator_2_finish_r) = unbounded();

    {
        let complete_receiving_buttons = Arc::clone(&complete_receiving_buttons);
        let button_press_queue = Arc::clone(&button_press_queue);
        let elevator_under_maintenence = Arc::clone(&elevator_under_maintenence);
        pool.execute(move || loop {
            let _ = receive_instructions(
                &button_press_queue,
                &complete_receiving_buttons,
                &elevator_under_maintenence,
            );
        });
    }

    {
        // Elevator 1 process request: Get all the people that need to fetch (Periodic Task)
        let elevator_name = "A";
        let elevator_1_current_floor = Arc::new(Mutex::new(0));
        let button_press_queue = Arc::clone(&button_press_queue);
        let elevator_requests_queue = Arc::new(Mutex::new(VecDeque::new()));
        let complete_receiving_buttons = Arc::clone(&complete_receiving_buttons);
        let elevator_under_maintenence = Arc::clone(&elevator_under_maintenence);

        let handle = {
            let elevator_requests_queue = Arc::clone(&elevator_requests_queue);
            let elevator_1_current_floor = Arc::clone(&elevator_1_current_floor);
            let complete = Arc::new(Mutex::new(false));

            scheduled_thread_pool.execute_at_fixed_rate(
                Duration::from_millis(5),
                Duration::from_millis(5),
                move || {
                    if *elevator_under_maintenence.lock().unwrap() == elevator_name {
                        elevator_1_request_s
                            .send(elevator_system::QueueStatus::Done)
                            .unwrap();
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

            pool.execute(move || loop {
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

            scheduled_thread_pool.execute_at_fixed_rate(
                Duration::from_millis(5),
                Duration::from_millis(5),
                move || {
                    if *elevator_under_maintenence.lock().unwrap() == elevator_name {
                        elevator_2_request_s
                            .send(elevator_system::QueueStatus::Done)
                            .unwrap();
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
            pool.execute(move || loop {
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
            });
        }
    }

    loop {
        if !(*complete_receiving_buttons.lock().unwrap()) {
            continue;
        } else if !elevator_1_finish_r.is_empty() && !elevator_2_finish_r.is_empty() {
            break;
        }
    }
}

fn receive_instructions(
    button_press_queue: &Mutex<VecDeque<ButtonPressed>>,
    complete_receiving_buttons: &Mutex<bool>,
    elevator_under_maintenance: &Mutex<String>,
) -> Result<()> {
    let mut connection: Connection =
        Connection::insecure_open("amqp://guest:guest@localhost:5672")?;

    let channel: amiquip::Channel = connection.open_channel(None)?;

    let queue = channel.queue_declare("hello", QueueDeclareOptions::default())?;

    let consumer = queue.consume(ConsumerOptions::default())?;
    println!("Elevators: Waiting for queue...");

    for message in consumer.receiver().iter() {
        match message {
            ConsumerMessage::Delivery(delivery) => {
                let body = String::from_utf8_lossy(&delivery.body);

                match serde_json::from_str::<Message>(&body) {
                    Ok(message) => match message {
                        Message::ButtonPressed(button_pressed) => {
                            button_press_queue.lock().unwrap().push_back(button_pressed);
                        }
                        Message::Complete(_status) => {
                            *complete_receiving_buttons.lock().unwrap() = true;
                        }
                        Message::ElevatorUnderMaintenance(elevator_under_maintain) => {
                            println!(
                                "*** ELEVATOR {} UNDER MAINTENANCE !!! ***: ",
                                elevator_under_maintain
                            );
                            *elevator_under_maintenance.lock().unwrap() = elevator_under_maintain;
                        }
                    },
                    Err(_) => {
                        // println!("Failed to deserialize message: {}", e);
                    }
                }

                consumer.ack(delivery)?;
            }
            other => {
                println!("Consumer ended: {:?}", other);
                break;
            }
        }
    }

    connection.close()
}

pub fn main() {
    benchmark!(1, {
        elevator_system();
    });
    let current_mem = PEAK_ALLOC.current_usage_as_kb();
    println!("\nThis program currently uses {} KB of RAM.", current_mem);
    let peak_mem = PEAK_ALLOC.peak_usage_as_kb();
    println!("The max amount that was used {}", peak_mem);
}
