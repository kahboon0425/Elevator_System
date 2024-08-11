use amiquip::{Connection, Exchange, Publish, Result};
use elevator_system::{ButtonPressed, Message};
use rand::Rng;
use scheduled_thread_pool;
use scheduled_thread_pool::ScheduledThreadPool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::{collections::VecDeque, time::Duration};

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

    let scheduled_thread_pool = ScheduledThreadPool::new(1);
    let complete_receiving_buttons = Arc::new(Mutex::new(false));

    let handle = {
        let complete_receiving_buttons = Arc::clone(&complete_receiving_buttons);

        // Periodic Task
        scheduled_thread_pool.execute_at_fixed_rate(
            Duration::from_millis(0),
            Duration::from_millis(3),
            move || {
                if let Some(button_pressed) = button_presses.pop_front() {
                    // serialize
                    let message_type = Message::ButtonPressed(button_pressed);

                    let serial_button_pressed = serde_json::to_string(&message_type).unwrap();
                    // send button pressed event through rabbit mq
                    let _ = send_msg(serial_button_pressed);
                } else if *complete_receiving_buttons.lock().unwrap() == false {
                    let message_type = Message::Complete(true);
                    let _ = send_msg(serde_json::to_string(&message_type).unwrap());
                    *complete_receiving_buttons.lock().unwrap() = true;
                }
            },
        )
    };

    loop {
        std::thread::sleep(Duration::from_secs(1));
        if *complete_receiving_buttons.lock().unwrap() == true {
            handle.cancel();
            break;
        }
    }
}

fn send_msg(directions: String) -> Result<()> {
    // Open connection.
    let mut connection = Connection::insecure_open("amqp://guest:guest@localhost:5672")?;

    // Open a channel - None says let the library choose the channel ID.
    let channel = connection.open_channel(None)?;

    // Get a handle to the direct exchange on our channel.
    let exchange = Exchange::direct(&channel);

    // Publish a message to the "hello" queue.
    exchange.publish(Publish::new(directions.as_bytes(), "hello"))?;

    connection.close()
}
