#![feature(try_blocks)]
mod gateway;
pub mod testing;

use self::gateway::{Frame, HelloOpCode, LoginOpCode};
use async_tungstenite::{tokio::connect_async as websocket_async, tungstenite::Message};
use futures::{sink::SinkExt, stream::StreamExt};
use reqwest::Client as HTTPClient;
use serde_json::{from_str as from_json, to_string as to_json};
use std::{pin::Pin, time::Duration};
use tokio::{join, select, sync::mpsc::{Receiver, Sender, channel}, time::delay_for as sleep};

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

	/*pub async fn gateway_start(&self) {
		let url = format!("wss://{}/socket", self.addresses.1);
		// Remove unwrap()!
		let mut socket = websocket_async(url).await.unwrap().0;

		let mut logged_in = false;
		let end = loop {
			// Remove second unwrap()!
			// Consider removing first unwrap(). (Can tungstenite return a None
			// before SocketClose?)
			let incoming_frame = socket.next().await.unwrap().unwrap();
			

			match incoming_frame {
				// Remove unwrap()!
				Message::Text(data) => match from_str(&data).unwrap() {
					Frame::Hello(HelloOpCode {heart_beat}) => {
						
					},
					_ => unimplemented!()
				},
				Message::Close(data) => break data,
				_ => unimplemented!()
			}
		};
		println!("what {:?}", end);
	}*/

	pub async fn start_gateway(&self) {
		let (outgoing_send, outgoing_receive) = channel(5);
		let (incoming_send, incoming_receive) = channel(5);

		join!(
			self.manage_gateway(incoming_send, outgoing_receive),
			self.listen_gateway(incoming_receive, outgoing_send)
		);
	}

	async fn manage_gateway(&self, mut sender: Sender<Frame>,
			mut receiver: Receiver<Option<Frame>>) {
		let url = format!("wss://{}/socket", self.addresses.1);
		let mut socket = websocket_async(url).await.unwrap().0; // Remove unwrap().

		loop {
			let incoming_frame = socket.next();
			let outgoing_frame = receiver.next();

			select! {
				// Remove second unwrap().
				// Consider removing first unwrap(). (Can tungstenite return a None
				// before SocketClose?)
				frame = incoming_frame => match frame.unwrap().unwrap() {
					// Remove unwrap()s.
					Message::Text(frame) => sender.send(
						from_json::<Frame>(&frame).unwrap().into_owned()).await.unwrap(),
					_ => unimplemented!() // Remove unimplemented!().
				},
				// Remove unwrap()s.
				frame = outgoing_frame => socket.send(Message::Text(
					to_json(&frame.flatten().unwrap()).unwrap())).await.unwrap()
			}
		}
	}

	async fn listen_gateway(&self, mut receiver: Receiver<Frame>,
			mut sender: Sender<Option<Frame>>) {
		// Remove unwrap().
		let incoming_frame = receiver.next().await.unwrap();

		let heart_beat = if let Frame::Hello(HelloOpCode {heart_beat})
				= incoming_frame {
			async move {
				let duration = Duration::from_millis(heart_beat.into());
				loop {
					// Remove unwrap().
					sleep(duration).await;
					sender.send(Some(Frame::HeartBeatAck)).await.unwrap();
				}
			}
		} else {unimplemented!()}; // Remove unimplemented!().
		heart_beat.await;

		loop {
			// Todo
			unimplemented!();
			println!("{:?}", incoming_frame);
		}
	}
}

/*pub fn do_sex() {
	use self::gateway::Frame;
	use serde_json::{from_str, to_string};

	let val: Result<(), serde_json::error::Error> = try {
		let serialized = "{\"op\":3}";
		println!("{}", serialized);
		let deserialized: Frame = from_str(serialized)?;
		println!("{:?}", deserialized);
		let serialized = to_string(&deserialized)?;
		println!("{}", serialized);
	};
	println!("{:?}", val);
}*/
