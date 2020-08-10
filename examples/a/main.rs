use hiven_rs::{Client, EventHandler, data::{House, Message}, gateway::EventInitState};
use tokio;

#[tokio::main]
async fn main() {
	let client = Client::new("token");
	client.start_gateway(MyEventHandler).await;
}

struct MyEventHandler;

impl EventHandler for MyEventHandler {
	fn on_connect(&self, event: EventInitState) {
		println!("I am @{}, also known as {}.", event.user.username, event.user.name);
		//println!("{:?}", event);
	}

	fn on_house_join(&self, event: House) {
		println!("I just joined a house named {}.", event.name);
		//println!("{:?}", event);
	}

	fn on_message(&self, event: Message) {
		println!("I just heard someone say {}.", event.content);
	}
}
