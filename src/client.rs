use self::super::{
	data::{House, Message},
	gateway::{
		EventInitState, EventTypingStart,
		Frame,
		OpCodeEvent, OpCodeHello, OpCodeLogin
	},
	http::{
		PathInfo,
		RequestInfo, RequestBodyInfo
	}
};
use async_tungstenite::{
	tokio::connect_async as websocket_async,
	tungstenite::Message as WebsocketMessage
};
use futures::{sink::SinkExt, stream::StreamExt};
use reqwest::Client as HTTPClient;
use serde_json::{from_str as from_json, to_string as to_json};
use std::{
	future::{Future, ready},
	pin::Pin,
	sync::Arc,
	thread::{JoinHandle, spawn},
	time::Duration
};
use tokio::{
	join, select,
	sync::mpsc::{Receiver, Sender, channel},
	time::delay_for as sleep
};

pub struct Client<'u, 't> {
	token: &'t str,
	domains: (&'u str, &'u str),
	http_client: HTTPClient
}

impl<'u, 't> Client<'u, 't> {
	pub fn new(token: &'t str) -> Self {
		Self {
			token: token,
			domains: ("api.hiven.io", "swarm-dev.hiven.io"),
			http_client: HTTPClient::new()
		}
	}

	pub fn new_at(token: &'t str, api_base: &'u str, gateway_base: &'u str) ->
			Self {
		Self {
			token: token,
			domains: (api_base, gateway_base),
			http_client: HTTPClient::new()
		}
	}

	pub async fn new_gate_keeper<'c, E>(&'c self, event_handler: E) ->
			GateKeeper<'c, 'u, 't, E>
				where E: EventHandler {
		GateKeeper::new(self, event_handler)
	}

	pub async fn start_gateway<E>(&self, event_handler: E)
			where E: EventHandler {
		let gate_keeper = GateKeeper::new(self, event_handler);
		gate_keeper.start_gateway().await;
	}

	pub async fn send_message<R>(&self, room: R, content: String)
			where R: Into<u64> {
		execute_request(&self.http_client, RequestInfo {
			token: self.token.to_owned(),
			path: PathInfo::MessageSend {
				channel_id: room.into()
			},
			body: RequestBodyInfo::MessageSend {
				content: content
			}
		}, self.domains.0).await;
	}
}

impl Client<'static, 'static> {
	pub fn start_gateway_later<E>(self: Arc<Self>, event_handler: E) ->
			JoinHandle<()>
				where E: EventHandler + 'static {
		spawn(move || {
			let gate_keeper = GateKeeper::new(&self, event_handler);
			let mut runtime = tokio::runtime::Runtime::new().unwrap();
			runtime.block_on(async {
				gate_keeper.start_gateway().await;
			});
		})
	}
}

async fn execute_request<'a>(client: &HTTPClient, request: RequestInfo,
		base_url: &'a str) {
	let path = format!("https://{}/v1{}", base_url, request.path.path());
	let http_request = client.request(request.body.method(), &path)
		.header("authorization", request.token);

	let http_request = if request.body.method() != "GET" {
		http_request.header("content-type", "application/json")
			.body(to_json(&request.body).unwrap()) // Remove unwrap().
	} else {http_request};
	
	// Remove unwrap()s.
	http_request.send().await.unwrap().error_for_status().unwrap();
}

pub struct GateKeeper<'c, 'u, 't, E>
		where E: EventHandler {
	pub client: &'c Client<'u, 't>,
	pub event_handler: E
}

impl<'c, 'u, 't, E> GateKeeper<'c, 'u, 't, E>
		where E: EventHandler {
	pub fn new(client: &'c Client<'u, 't>, event_handler: E) -> Self {
		Self {
			client: client,
			event_handler: event_handler
		}
	}

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
		let url = format!("wss://{}/socket", self.client.domains.1);
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
						// Err(err) => println!("{:?}: {}", err, frame),
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
							self.event_handler.on_connect(&self.client, data).await,
						OpCodeEvent::HouseJoin(data) =>
							self.event_handler.on_house_join(&self.client, data).await,
						OpCodeEvent::TypingStart(data) =>
							self.event_handler.on_typing(&self.client, data).await,
						OpCodeEvent::MessageCreate(data) =>
							self.event_handler.on_message(&self.client, data).await
					},
					_ => unimplemented!() // Remove unimplemented!().
				}
			}
		};

		join!(heart_beat, listener);
	}
}

pub trait EventHandler: Send {
	fn on_connect<'c>(&self, _client: &'c Client<'c, 'c>, _event: EventInitState) -> Pin<Box<dyn Future<Output = ()> + 'c>> {
		// NoOp
		Box::pin(ready(()))
	}

	fn on_house_join<'c>(&self, _client: &'c Client<'c, 'c>, _event: House) -> Pin<Box<dyn Future<Output = ()> + 'c>> {
		// NoOp
		Box::pin(ready(()))
	}

	fn on_typing<'c>(&self, _client: &'c Client<'c, 'c>, _event: EventTypingStart) -> Pin<Box<dyn Future<Output = ()> + 'c>> {
		// NoOp
		Box::pin(ready(()))
	}

	fn on_message<'c>(&self, _client: &'c Client<'c, 'c>, _event: Message) -> Pin<Box<dyn Future<Output = ()> + 'c>> {
		// NoOp
		Box::pin(ready(()))
	}
}
