use async_trait::async_trait;
use hiven_rs::{Client, EventHandler, data::{House, Message}, gateway::EventInitState};
use tokio;

#[tokio::main]
async fn main() {
	let client = Client::new("token");
	client.start_gateway(MyEventHandler).await;
}

struct MyEventHandler;

#[async_trait]
impl EventHandler for MyEventHandler {
	async fn on_connect(&self, _client: &'_ Client<'_, '_>, event: EventInitState) {
		println!("I am @{}, also known as {}.", event.user.username, event.user.name);
	}

	async fn on_house_join(&self, _client: &'_ Client<'_, '_>, event: House) {
		println!("I just joined a house named {}.", event.name);
	}

	async fn on_message(&self, client: &'_ Client<'_, '_>, event: Message) {
		println!("I just heard someone say {}.", event.content);

		if event.content.starts_with("$") {match &event.content[1..] {
			"hello" => {
				println!("I'm going to say hello back!");
				client.send_message(event.room_id, "Hello!".to_owned()).await
			},
			_ => ()
		}}
	}
}
