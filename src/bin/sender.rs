use amiquip::{Connection, Exchange, Publish, Result};
use crossbeam_channel;
use elevator_system::{ButtonPressed, Data, Elevator, QueueStatus};
use scheduled_thread_pool;
use scheduled_thread_pool::ScheduledThreadPool;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, MutexGuard};
use std::{collections::VecDeque, time::Duration};

fn main() {
    let scheduled_thread_pool = ScheduledThreadPool::new(3);
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

    let scheduled_thread_pool = ScheduledThreadPool::new(3);

    let handle = {
        // let complete_receiving_buttons = Arc::clone(&complete_receiving_buttons);

        // Periodic Task
        scheduled_thread_pool.execute_at_fixed_rate(
            Duration::from_millis(0),
            Duration::from_millis(1000),
            move || {
                if let Some(button_pressed) = button_presses.pop_front() {
                    // println!("Button Pressed: {:?}", button_pressed);
                    // // serialize
                    let serial_button_pressed = serde_json::to_string(&button_pressed).unwrap();
                    // // send button pressed event through rabbit mq
                    let _ = send_msg(serial_button_pressed);
                    // let _ = send_msg("hello".to_string());
                } else {
                    // *complete_receiving_buttons.lock().unwrap() = true;
                }
            },
        )
    };
    loop {}
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
