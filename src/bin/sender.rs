use amiquip::{Connection, Exchange, Publish, Result};
use elevator_system::{ButtonPressed, ElevatorEvent, Message};
use rand::Rng;
use std::sync::mpsc::channel;
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

    let pool = ThreadPool::new(3);
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

            // Randomly trigger the broken time for elevator
            let random_time = rng.gen_range(10..20);

            thread::sleep(Duration::from_millis(random_time * 100));

            // Send message to elevator controller when the elevator is broken
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
                // Generate people arrive at random time
                let mut rng = rand::thread_rng();
                let people_arrival_time = rng.gen_range(1..8);
                thread::sleep(Duration::from_millis(people_arrival_time * 100));

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
                            let message_type = Message::ElevatorUnderMaintenance(elevator);
                            let serial_maintenance_elevator =
                                serde_json::to_string(&message_type).unwrap();

                            // Send maintanence event through RabbitMQ
                            let _ = send_msg(serial_maintenance_elevator);
                        }
                        ElevatorEvent::ButtonPress(button_pressed) => {
                            // Serialize the message
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
                            println!("Elevator Controller: No more people");
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

    pool.join();
}

fn send_msg(button_pressed: String) -> Result<()> {
    let mut connection = Connection::insecure_open("amqp://guest:guest@localhost:5672")?;

    let channel = connection.open_channel(None)?;

    let exchange = Exchange::direct(&channel);

    exchange.publish(Publish::new(button_pressed.as_bytes(), "hello"))?;

    connection.close()
}
