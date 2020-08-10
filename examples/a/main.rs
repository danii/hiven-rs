use hiven_rs::Client;
use tokio;

#[tokio::main]
async fn main() {
	let client = Client::new("token");
	client.start_gateway().await;
	//hiven_rs::do_sex();
}
