use self::super::{
	data::{House, Message},
	gateway::{
		EventInitState, EventTypingStart,
		Frame,
		OpCodeEvent, OpCodeHello, OpCodeLogin
	}
};
use async_tungstenite::{
	tokio::connect_async as websocket_async,
	tungstenite::Message as WebsocketMessage
};
use futures::{sink::SinkExt, stream::StreamExt};
//use reqwest::Client as HTTPClient;
use serde_json::{from_str as from_json, to_string as to_json};
use std::time::Duration;
use tokio::{
	join, select,
	sync::mpsc::{Receiver, Sender, channel},
	time::delay_for as sleep
};

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

	pub async fn start_gateway<E>(&self, event_handler: E)
			where E: EventHandler {
		let gate_keeper = GateKeeper {
			client: self,
			event_handler: event_handler
		};

		gate_keeper.start_gateway().await;
	}
}

pub struct GateKeeper<'c, 'u, 't, E>
		where E: EventHandler {
	pub client: &'c Client<'u, 't>,
	pub event_handler: E
}

impl<'c, 'u, 't, E> GateKeeper<'c, 'u, 't, E>
		where E: EventHandler{
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
		let url = format!("wss://{}/socket", self.client.addresses.1);
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
					WebsocketMessage::Text(frame) => match from_json::<Frame>(&frame) {
						Ok(frame) => sender.send(frame).await.unwrap(),
						// Uncomment to show events that can't yet be parsed.
						Err(err) => println!("{:?}: {}", err, frame),
						_ => ()
					},
					_ => unimplemented!("B") // Remove unimplemented!().
				},
				// Remove unwrap()s.
				frame = outgoing_frame => socket.send(WebsocketMessage::Text(
					to_json(&frame.flatten().unwrap()).unwrap())).await.unwrap()
			}
		}
	}

	async fn listen_gateway(&self, mut receiver: Receiver<Frame>,
			mut sender: Sender<Option<Frame>>) {
		// Remove unwrap().
		let incoming_frame = receiver.next().await.unwrap();

		let heart_beat =
			if let Frame::Hello(OpCodeHello {heart_beat}) = incoming_frame {
				let mut sender = sender.clone();
				let duration = Duration::from_millis(heart_beat.into());
				async move {
					loop {
						// Remove unwrap().
						sleep(duration).await;
						sender.send(Some(Frame::HeartBeat)).await.unwrap();
					}
				}
			} else {
				// Unexpected response...
				unimplemented!("A"); // Remove unimplemented!().
			};

		let frame = Frame::Login(OpCodeLogin {token: self.client.token.to_owned()});
		sender.send(Some(frame)).await.unwrap(); // Remove unwrap().

		let listener = async {
			loop {
				// Remove unwrap().
				match receiver.next().await.unwrap() {
					Frame::Event(event) => match event {
						OpCodeEvent::InitState(data) =>
							self.event_handler.on_connect(data),
						OpCodeEvent::HouseJoin(data) =>
							self.event_handler.on_house_join(data),
						OpCodeEvent::TypingStart(data) =>
							self.event_handler.on_typing(data),
						OpCodeEvent::MessageCreate(data) =>
							self.event_handler.on_message(data)
					},
					_ => unimplemented!() // Remove unimplemented!().
				}
			}
		};

		join!(heart_beat, listener);
	}
}

pub trait EventHandler {
	fn on_connect(&self, _event: EventInitState) {/* NoOp */}
	fn on_house_join(&self, _event: House) {/* NoOp */}
	fn on_typing(&self, _event: EventTypingStart) {/* NoOp */}
	fn on_message(&self, _event: Message) {/* NoOp */}
}
