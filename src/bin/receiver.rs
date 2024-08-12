use std::{
    collections::VecDeque,
    sync::{mpsc::channel, Arc, Mutex},
    time::Duration,
};

use amiquip::{Connection, ConsumerMessage, ConsumerOptions, QueueDeclareOptions, Result};
use crossbeam_channel::unbounded;
use elevator_system::{elevator_handle_request, process_request, ButtonPressed, Message};
use scheduled_thread_pool::ScheduledThreadPool;
use threadpool::ThreadPool;

fn main() {
    let button_press_queue = Arc::new(Mutex::new(VecDeque::new()));
    let complete_receiving_buttons = Arc::new(Mutex::new(false));
    let scheduled_thread_pool = ScheduledThreadPool::new(3);
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
        // Elevator 1 handle task
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
                    process_request(
                        &elevator_name,
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
                    &elevator_name,
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
    // Elevator 2 process requests - Periodic Task

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
                    process_request(
                        &elevator_name,
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
        // Elevator 2 handling request - Aperiodic Task
        {
            let elevator_requests_queue = Arc::clone(&elevator_requests_queue);
            let elevator_2_current_floor = Arc::clone(&elevator_2_current_floor);
            pool.execute(move || loop {
                if elevator_handle_request(
                    &elevator_name,
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

    // pool.join()

    loop {
        if *complete_receiving_buttons.lock().unwrap() == false {
            continue;
        } else {
            // handle.cancel();
            if elevator_1_finish_r.is_empty() == false && elevator_2_finish_r.is_empty() == false {
                break;
            }
        }
    }
}

fn receive_instructions(
    button_press_queue: &Mutex<VecDeque<ButtonPressed>>,
    complete_receiving_buttons: &Mutex<bool>,
    elevator_under_maintenance: &Mutex<String>,
) -> Result<()> {
    // Open connection.
    let mut connection: Connection =
        Connection::insecure_open("amqp://guest:guest@localhost:5672")?;

    // Open a channel - None says let the library choose the channel ID.
    let channel: amiquip::Channel = connection.open_channel(None)?;

    // Declare the "hello" queue.
    let queue = channel.queue_declare("hello", QueueDeclareOptions::default())?;

    // Start a consumer.
    let consumer = queue.consume(ConsumerOptions::default())?;
    println!("Elevator: Waiting for queue...");

    for (i, message) in consumer.receiver().iter().enumerate() {
        match message {
            ConsumerMessage::Delivery(delivery) => {
                let body = String::from_utf8_lossy(&delivery.body);
                // println!("{}", body);

                match serde_json::from_str::<Message>(&body) {
                    Ok(message) => match message {
                        Message::ButtonPressed(button_pressed) => {
                            // println!("Button pressed: {:?}", button_pressed);
                            button_press_queue.lock().unwrap().push_back(button_pressed);
                        }
                        Message::Complete(status) => {
                            // println!("Receive Complete!!!!!!!!!!!!!!!!!!!");
                            *complete_receiving_buttons.lock().unwrap() = true;
                        }
                        Message::ElevatorUnderMaintenance(elevator_under_maintain) => {
                            println!(
                                "Elevator_under_maintainnnnnnnnnnnnnnnnn: {}",
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
