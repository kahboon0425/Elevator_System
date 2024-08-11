use std::{
    collections::VecDeque,
    sync::{mpsc::channel, Arc, Mutex, MutexGuard},
    time::Duration,
};

use amiquip::{Connection, ConsumerMessage, ConsumerOptions, QueueDeclareOptions, Result};
use crossbeam_channel::unbounded;
use elevator_system::{
    elevator_handle_request, process_request, ButtonPressed, Elevator, Message, QueueStatus,
};
use scheduled_thread_pool::ScheduledThreadPool;
use threadpool::ThreadPool;

fn main() {
    let button_press_queue = Arc::new(Mutex::new(VecDeque::new()));
    let complete_receiving_buttons = Arc::new(Mutex::new(false));
    let scheduled_thread_pool = ScheduledThreadPool::new(3);
    let pool = ThreadPool::new(2);

    let (elevator_1_request_s, elevator_1_request_r) = channel();
    let (elevator_2_request_s, elevator_2_request_r) = channel();
    let (elevator_1_finish_s, elevator_1_finish_r) = unbounded();
    let (elevator_2_finish_s, elevator_2_finish_r) = unbounded();

    {
        let complete_receiving_buttons = Arc::clone(&complete_receiving_buttons);
        let button_press_queue = Arc::clone(&button_press_queue);
        pool.execute(move || loop {
            let _ = receive_instructions(&button_press_queue, &complete_receiving_buttons);
        });
    }

    {
        // Elevator 1 handle task
        let elevator_name = "A";
        let elevator_1_current_floor = Arc::new(Mutex::new(0));
        let button_press_queue = Arc::clone(&button_press_queue);
        let elevator_requests_queue = Arc::new(Mutex::new(VecDeque::new()));
        let complete_receiving_buttons = Arc::clone(&complete_receiving_buttons);

        let handle = {
            let elevator_requests_queue = Arc::clone(&elevator_requests_queue);
            let elevator_1_current_floor = Arc::clone(&elevator_1_current_floor);
            let complete = Arc::new(Mutex::new(false));

            scheduled_thread_pool.execute_at_fixed_rate(
                Duration::from_millis(5),
                Duration::from_millis(5),
                move || {
                    process_request(
                        &elevator_name,
                        &elevator_1_current_floor,
                        &button_press_queue,
                        &elevator_requests_queue,
                        &elevator_1_request_s,
                        &complete_receiving_buttons,
                        &complete,
                    );
                    // let elevator_1 = Elevator::new_elevator(
                    //     "A".to_string(),
                    //     *elevator_1_current_floor.lock().unwrap(),
                    // );
                    // if let Some(request_queue) =
                    //     elevator_1.process_requests(button_press_queue.lock().unwrap())
                    // {
                    //     let mut elevator_requests_queue = elevator_requests_queue.lock().unwrap();
                    //     let request_queue_count = request_queue.len();
                    //     elevator_requests_queue.extend(request_queue);
                    //     // println!("Elevator A Request Queue: {:?}", elevator_requests_queue);

                    //     elevator_1_request_s
                    //         .send(QueueStatus::NewQueue(request_queue_count))
                    //         .unwrap();
                    // } else {
                    //     match *complete_receiving_buttons.lock().unwrap() {
                    //         true => {
                    //             if *complete.lock().unwrap() == false {
                    //                 elevator_1_request_s.send(QueueStatus::Done).unwrap();
                    //                 *complete.lock().unwrap() = true;
                    //             } else {
                    //                 elevator_1_request_s.send(QueueStatus::Empty).unwrap()
                    //             }
                    //         }
                    //         false => {
                    //             // println!("Done message already sent!");
                    //         }
                    //     }
                    // }
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
                // let mut elevator_1 = Elevator::new_elevator(
                //     "A".to_string(),
                //     *elevator_1_current_floor.lock().unwrap(),
                // );

                // if let Ok(queue_status) = elevator_1_request_r.recv() {
                //     match queue_status {
                //         QueueStatus::NewQueue(request_queue_count) => {
                //             let mut request_queue = elevator_requests_queue.lock().unwrap();
                //             println!(
                //                 "\tElevator {} handle request of person {:?}",
                //                 elevator_1.id,
                //                 request_queue
                //                     .iter()
                //                     .take(request_queue_count)
                //                     .map(|r| r.person_id)
                //                     .collect::<Vec<_>>()
                //             );
                //             let elevator_current_floor =
                //                 elevator_1.handle_requests(&request_queue, request_queue_count);

                //             *request_queue = request_queue.split_off(request_queue_count);
                //             *elevator_1_current_floor.lock().unwrap() = elevator_current_floor;
                //         }
                //         // Do nothing
                //         QueueStatus::Empty => {}
                //         QueueStatus::Done => {
                //             handle.cancel();
                //             elevator_1_finish_s.send(()).unwrap();
                //             break;
                //         }
                //     }
                // }
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
                    process_request(
                        &elevator_name,
                        &elevator_2_current_floor,
                        &button_press_queue,
                        &elevator_requests_queue,
                        &elevator_2_request_s,
                        &complete_receiving_buttons,
                        &complete,
                    );

                    // let elevator_2 = Elevator::new_elevator(
                    //     "B".to_string(),
                    //     *elevator_2_current_floor.lock().unwrap(),
                    // );
                    // if let Some(request_queue) =
                    //     elevator_2.process_requests(button_press_queue.lock().unwrap())
                    // {
                    //     let mut elevator_requests_queue = elevator_requests_queue.lock().unwrap();
                    //     let request_queue_count = request_queue.len();
                    //     elevator_requests_queue.extend(request_queue);

                    //     elevator_2_request_s
                    //         .send(QueueStatus::NewQueue(request_queue_count))
                    //         .unwrap();
                    // } else {
                    //     match *complete_receiving_buttons.lock().unwrap() {
                    //         true if *complete.lock().unwrap() == false => {
                    //             elevator_2_request_s.send(QueueStatus::Done).unwrap();
                    //             *complete.lock().unwrap() = true;
                    //         }
                    //         false => elevator_2_request_s.send(QueueStatus::Empty).unwrap(),
                    //         true => {
                    //             // println!("Done message already sent!");
                    //         }
                    //     }
                    // }
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
                // let mut elevator_2 = Elevator::new_elevator(
                //     "B".to_string(),
                //     *elevator_2_current_floor.lock().unwrap(),
                // );
                // if let Ok(queue_status) = elevator_2_request_r.recv() {
                //     match queue_status {
                //         QueueStatus::NewQueue(request_queue_count) => {
                //             let mut request_queue = elevator_requests_queue.lock().unwrap();
                //             println!(
                //                 "\tElevator {} handle request of person {:?}",
                //                 elevator_2.id,
                //                 request_queue
                //                     .iter()
                //                     .map(|r| r.person_id)
                //                     .collect::<Vec<_>>()
                //             );
                //             let elevator_current_floor =
                //                 elevator_2.handle_requests(&request_queue, request_queue_count);

                //             *request_queue = request_queue.split_off(request_queue_count);
                //             *elevator_2_current_floor.lock().unwrap() = elevator_current_floor;
                //         }
                //         QueueStatus::Empty => {}
                //         QueueStatus::Done => {
                //             handle.cancel();
                //             elevator_2_finish_s.send(()).unwrap();
                //             break;
                //         }
                //     }
                // }
            });
        }
    }

    // pool.join()

    loop {
        if !(*complete_receiving_buttons.lock().unwrap()) {
            continue;
        } else {
            // handle.cancel();
            if !elevator_1_finish_r.is_empty() && !elevator_2_finish_r.is_empty() {
                break;
            }
        }
    }
    // let _ = receive_instructions(button_press_queue.clone());
}

fn receive_instructions(
    button_press_queue: &Mutex<VecDeque<ButtonPressed>>,
    complete_receiving_buttons: &Mutex<bool>,
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
                            println!(
                                "Person {} press lift button at floor {} to floor {} *****",
                                button_pressed.person_id,
                                button_pressed.current_floor,
                                button_pressed.target_floor
                            );
                        }
                        Message::Complete(status) => {
                            // println!("Complete status: {}", status);
                            *complete_receiving_buttons.lock().unwrap() = true;
                        }
                    },
                    Err(e) => {
                        println!("Failed to deserialize message: {}", e);
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
