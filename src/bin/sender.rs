use amiquip::{Connection, Exchange, Publish, Result};
use elevator_system::{ButtonPressed, ElevatorEvent, Message};
use rand::Rng;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::{collections::VecDeque, time::Duration};
use threadpool::ThreadPool;

fn main() {
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

    // let scheduled_thread_pool = ScheduledThreadPool::new(1);
    let pool = ThreadPool::new(2);
    let complete_receiving_buttons = Arc::new(Mutex::new(false));
    let (event_sender, event_receiver) = channel();

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

            let random_time = rng.gen_range(5..10);
            println!("Random Broken Time: {}", random_time);

            thread::sleep(Duration::from_millis(random_time * 10));

            println!("Elevator {} broken:", elevator_chosen);

            event_sender
                .send(ElevatorEvent::Maintenance(elevator_chosen.to_string()))
                .unwrap();
        })
    }

    {
        // Button Press Event
        let event_sender = event_sender.clone();

        pool.execute(move || loop {
            if let Some(button_pressed) = button_presses.pop_front() {
                let mut rng = rand::thread_rng();
                let people_arrival_time = rng.gen_range(1..8);
                thread::sleep(Duration::from_millis(people_arrival_time * 10));

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
        pool.execute(move || loop {
            match event_receiver.recv() {
                Ok(event) => {
                    match event {
                        ElevatorEvent::Maintenance(elevator) => {
                            println!(
                                "Elevator Controller: Elevator {} Entering maintenance mode.",
                                elevator
                            );
                            let message_type = Message::ElevatorUnderMaintenance(elevator);
                            let serial_maintenance_elevator =
                                serde_json::to_string(&message_type).unwrap();

                            // Send maintanence event through RabbitMQ
                            let _ = send_msg(serial_maintenance_elevator);
                        }
                        ElevatorEvent::ButtonPress(button_pressed) => {
                            // serialize
                            let message_type = Message::ButtonPressed(button_pressed);
                            println!(
                                "Person {} press lift button at floor {} to floor {} *****",
                                button_pressed.person_id,
                                button_pressed.current_floor,
                                button_pressed.target_floor
                            );

                            let serial_button_pressed =
                                serde_json::to_string(&message_type).unwrap();

                            // Send button pressed event through RabbitMQ
                            let _ = send_msg(serial_button_pressed);
                        }
                        ElevatorEvent::Complete => {
                            println!("Elevator Controller: Done...");
                            let message_type = Message::Complete(true);
                            let _ = send_msg(serde_json::to_string(&message_type).unwrap());
                            break;
                        }
                    }
                }
                Err(_) => println!("Nothing yet..."),
            }
        });
    }

    // loop {
    //     if *complete_receiving_buttons.lock().unwrap() == true {
    //         // handle.cancel();
    //         break;
    //     }
    // }
    pool.join();
}

fn send_msg(button_pressed: String) -> Result<()> {
    let mut connection = Connection::insecure_open("amqp://guest:guest@localhost:5672")?;

    let channel = connection.open_channel(None)?;

    let exchange = Exchange::direct(&channel);

    exchange.publish(Publish::new(button_pressed.as_bytes(), "hello"))?;

    connection.close()
}
