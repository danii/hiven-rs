use reqwest::Client as HTTPClient;
use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::Message;
use futures::sink::SinkExt;
use futures::stream::StreamExt;

pub async fn do_sex() {
	let mut socket = connect_async("wss://swarm-dev.hiven.io/socket?encoding=json&compression=text_json").await.unwrap().0;

	loop {
		let response = match socket.next().await {
			None => {
				println!("exiting");
				break
			},
			Some(data) => {
				println!("{:?}", data);
				data.unwrap().into_text().unwrap()
			}
		};
		println!("incoming: {}", response);

		if response.starts_with("{\"op\":1") {
			socket.send(Message::text("{\"op\":2,\"d\":{\"token\":\"token\"}}")).await;
			println!("sent op 2");
		}
	}
	println!("socket closed!");
}

pub async fn send_message(token: String, channel_id: u64, content: String) {
	let client = HTTPClient::new();
	let url = format!("https://api.hiven.io/v1/rooms/{}/messages", channel_id);
	let request = client.post(&url)
		.header("content-type", "application/json")
		.header("authorization", token)
		.body(format!("{{\"content\":\"{}\"}}", content))
		.send().await;
}
