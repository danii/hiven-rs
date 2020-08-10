use hiven_rs::{Client, EventHandler};
use tokio;

#[tokio::main]
async fn main() {
	let client = Client::new("token");
	client.start_gateway(MyEventHandler).await;
	//hiven_rs::do_sex();
}

struct MyEventHandler;

impl EventHandler for MyEventHandler {}
