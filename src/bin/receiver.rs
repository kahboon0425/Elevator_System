use amiquip::{Connection, ConsumerMessage, ConsumerOptions, QueueDeclareOptions, Result};
use elevator_system::ButtonPressed;

fn main() {
    let _ = receive_instructions();
}

fn receive_instructions() -> Result<()> {
    // Open connection.
    let mut connection: Connection =
        Connection::insecure_open("amqp://guest:guest@localhost:5672")?;

    // Open a channel - None says let the library choose the channel ID.
    let channel: amiquip::Channel = connection.open_channel(None)?;

    // Declare the "hello" queue.
    let queue = channel.queue_declare("hello", QueueDeclareOptions::default())?;

    // Start a consumer.
    let consumer = queue.consume(ConsumerOptions::default())?;
    println!("VEHICLE: Waiting for instructions...");

    for (i, message) in consumer.receiver().iter().enumerate() {
        match message {
            ConsumerMessage::Delivery(delivery) => {
                let body = String::from_utf8_lossy(&delivery.body);
                // println!("{}", body);
                let deserialize_button_pressed: ButtonPressed =
                    serde_json::from_str(&body).unwrap();
                println!("{:?}", deserialize_button_pressed);
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
