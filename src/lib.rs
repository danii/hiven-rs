mod gateway;

use async_tungstenite::{tokio::connect_async as websocket_async, tungstenite::Message};
use futures::{sink::SinkExt, stream::StreamExt};
use reqwest::Client as HTTPClient;

enum RequestInfo {
	MessageSend {
		channel_id: u64,
		content: String
	},
	MessageEdit {
		channel_id: u64,
		message_id: u64,
		new_content: String
	},
	MessageDelete {
		channel_id: u64,
		message_id: u64
	}
}

impl RequestInfo {
	fn get_path(&self) -> String {
		match self {
			Self::MessageSend {channel_id, ..} =>
				format!("/rooms/{}/messages", channel_id),
			Self::MessageEdit {channel_id, message_id, ..} |
			Self::MessageDelete {channel_id, message_id, ..} =>
				format!("/rooms/{}/messages/{}", channel_id, message_id),
		}
	}

	fn get_method(&self) -> &'static str {
		match self {
			Self::MessageSend {..} => "POST",
			Self::MessageEdit {..} => "PATCH",
			Self::MessageDelete {..} => "DELETE"
		}
	}
}

pub struct Client<'u, 't> {
	addresses: (&'u str, &'u str),
	token: &'t str
}

impl<'u, 't> Client<'u, 't> {
	pub fn new(token: &'t str) -> Self {
		Self {
			addresses: ("api.hiven.io", "swarm-dev.hiven.io"),
			token: token
		}
	}

	pub async fn start_gateway(&self) {
		let url = format!("wss://{}/socket", self.addresses.1);
		let mut socket = websocket_async(url);

		loop {
			
		}
	}
}
