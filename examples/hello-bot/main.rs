use hiven_rs::{Client, EventHandler, client::Error as ClientError, data::{House, Message}, gateway::EventInitState};
use std::{future::Future, pin::Pin};
use tokio::time::delay_for;

#[tokio::main]
async fn main() -> Result<(), ClientError> {
	let client = Client::new("token");
	client.start_gateway(MyEventHandler).await
}

struct MyEventHandler;

impl EventHandler for MyEventHandler {
	fn on_connect<'c>(&self, _client: &'c Client, event: EventInitState) ->
			Pin<Box<dyn Future<Output = ()> + 'c>> {
		let output = async move {
			println!("I am @{}, also known as {}.", event.user.username, event.user.name);
			//println!("{:?}", event);
		};

		Box::pin(output)
	}

	fn on_house_join<'c>(&self, _client: &'c Client, event: House) ->
			Pin<Box<dyn Future<Output = ()> + 'c>> {
		let output = async move {
			println!("I just joined a house named {}.", event.name);
			//println!("{:?}", event);
		};

		Box::pin(output)
	}

	fn on_message<'c>(&self, client: &'c Client, event: Message) ->
			Pin<Box<dyn Future<Output = ()> + 'c>> {
		let output = async move {
			println!("I just heard someone say {}.", event.content);

			if event.content.starts_with("$") {match &event.content[1..] {
				"hello" => {
					println!("I'm going to say hello back!");

					client.trigger_typing(event.room_id).await;
					delay_for(std::time::Duration::from_millis(1000)).await;
					client.send_message(event.room_id, "Hello!".to_owned()).await
				},
				_ => ()
			}}
		};

		Box::pin(output)
	}
}
