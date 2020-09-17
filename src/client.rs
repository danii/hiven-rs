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
	tungstenite::{
		Message as WebsocketMessage,
		protocol::frame::CloseFrame
	}
};
use futures::{sink::SinkExt, stream::StreamExt};
use reqwest::Client as HTTPClient;
use serde_json::{from_str as from_json, to_string as to_json};
use std::{
	fmt::Debug,
	future::{Future, ready},
	pin::Pin,
	result::Result as STDResult,
	sync::Arc,
	thread::{JoinHandle, spawn},
	time::Duration
};
use tokio::{
	join, select,
	sync::{Notify, mpsc::{Receiver, Sender, channel, error::SendError}},
	time::timeout
};

type Result<T> = STDResult<T, Error>;

/// Authentication of a user on hiven.
///
/// With authentication of a user, you can call API endpoints as that user, or
/// start a gateway connection.
///
/// Getting Your Token
/// ------------------
/// To be able to authenticate a user, you must have a token. To get a token of
/// a user, you must be logged in as them in the browser. These are the steps
/// to get your token if you are logged in:
/// - Go to [app.hiven.io](https://app.hiven.io/)
/// - Enter any room that you have permission to speak in
/// - Press CTRL+SHIFT+I, opening up Developer Tools
/// - Go to the network tab on the Developer Tools window
/// - Start typing in the room
/// - Select the new `typing` request that appears. If two show up, select
/// 	the one with a 200 status code
/// - Look for the `authorization` header under Request Headers, under Headers
/// - The long string to the right is your token
///
/// Please remember, tokens should be treated exactly like passwords. **Never
/// give out your token, and if you do, only give it to people you would trust
/// with your password.** Another thing to keep in mind; it's always good
/// etiquette to automate seperate accounts, dedicated for automation, rather
/// than your own.
pub struct Client<'u, 't> {
	token: &'t str,
	domains: (&'u str, &'u str),
	http_client: HTTPClient
}

impl<'u, 't> Client<'u, 't> {
	/// Creates a new client with an authentication token. Uses the official
	/// hiven.io servers.
	pub fn new(token: &'t str) -> Self {
		Self {
			token: token,
			domains: ("api.hiven.io", "swarm-dev.hiven.io"),
			http_client: HTTPClient::new()
		}
	}

	/// Creates a new client with an authentication token, allows you to specify
	/// a base domain for the api and gateway.
	pub fn new_at(token: &'t str, api_base: &'u str, gateway_base: &'u str) ->
			Self {
		Self {
			token,
			domains: (api_base, gateway_base),
			http_client: HTTPClient::new()
		}
	}

	pub async fn new_gate_keeper<'c, E>(&'c self, event_handler: E) ->
			GateKeeper<'c, 'u, 't, E>
				where E: EventHandler {
		GateKeeper::new(self, event_handler)
	}

	/// Takes control of this thread, starting a connection to the gateway and
	/// dispatching gateway events asynchronously.
	///
	/// This method takes an event handler to handle all gateway events. Gateway
	/// events you do not implement will default to a method that does nothing
	/// (NoOp). Due to limitations with traits (and the async_trait macro), event
	/// handlers are not marked as `async`, but are asynchronous in spirit.
	/// Implementing an event listener can be done like this...
	/// ```rust
	/// use hiven_rs::{client::{Client, EventHandler}, data::Message};
	/// use std::{future::Future, pin::Pin};
	///
	/// // ...
	///
	/// # struct MyEventHandler;
	/// #
	/// impl EventHandler for MyEventHandler {
	/// 	fn on_message<'c>(&self, client: &'c Client, event: Message) ->
	/// 			Pin<Box<dyn Future<Output = ()> + 'c>> {
	/// 		Box::pin(async move {
	/// 			// Asynchronous code goes here.
	/// 		})
	/// 	}
	/// }
	/// ```
	///
	/// This method currently is not expected to return ever, unless during panic
	/// unwinding, and it's return value should be treated as `!` (the never
	/// type).
	//
	// Notes For Next Minor Version
	// ----------------------------
	// In the next minor version, this method will be renamed and it's signature
	// will be changed to support returning of Errors, or a unit, with
	// the introduction of a stop function.
	pub async fn start_gateway<E>(&self, event_handler: E) -> Result<()>
			where E: EventHandler {
		let gate_keeper = GateKeeper::new(self, event_handler);
		gate_keeper.start_gateway().await
	}

	pub async fn send_message<R>(&self, room: R, content: String)
			where R: Into<u64> {
		execute_request(&self.http_client, RequestInfo {
			token: self.token.to_owned(),
			path: PathInfo::MessageSend {
				channel_id: room.into()
			},
			body: RequestBodyInfo::MessageSend {content}
		}, self.domains.0).await;
	}

	pub async fn trigger_typing<R>(&self, room: R)
			where R: Into<u64> {
		execute_request(&self.http_client, RequestInfo {
			token: self.token.to_owned(),
			path: PathInfo::TypingTrigger {
				channel_id: room.into()
			},
			body: RequestBodyInfo::TypingTrigger {}
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
				gate_keeper.start_gateway().await.unwrap();
			});
		})
	}
}

async fn execute_request(client: &HTTPClient, request: RequestInfo,
		base_url: &str) {
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

// These lifetimes and this generic are a special set of generics, they are able
// to describe the person reading them with 100% accuracy.
//
// I promise this wasn't intended, but now I love it.
pub struct GateKeeper<'c, 'u, 't, E>
		where E: EventHandler {
	pub client: &'c Client<'u, 't>,
	pub event_handler: E
}

impl<'c, 'u, 't, E> GateKeeper<'c, 'u, 't, E>
		where E: EventHandler {
	pub fn new(client: &'c Client<'u, 't>, event_handler: E) -> Self {
		Self {client, event_handler}
	}

	pub async fn start_gateway(&self) -> Result<()> {
		let (outgoing_send, outgoing_receive) = channel(5);
		let (incoming_send, incoming_receive) = channel(5);

		match join!(
			self.manage_gateway(incoming_send, outgoing_receive),
			self.listen_gateway(incoming_receive, outgoing_send)
		) {
			(Ok(()), Ok(())) => Ok(()),
			(Err(err), Ok(())) => Err(err),
			(Ok(()), Err(err)) => Err(err),
			(Err(_), Err(err)) => Err(err)
		}
	}

	async fn manage_gateway(&self, mut sender: Sender<Frame>,
			mut receiver: Receiver<Option<Frame>>) -> Result<()> {
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
					WebsocketMessage::Close(close_data) => return Err(Error::socket_close(close_data)),
					frame => return Err(Error::expectation_failed(
						"Text or Close frames only", frame))
				},
				// Remove unwrap()s.
				frame = outgoing_frame => socket.send(WebsocketMessage::Text(
					to_json(&frame.flatten().unwrap()).unwrap())).await.unwrap()
			}
		}
	}

	async fn listen_gateway(&self, mut receiver: Receiver<Frame>,
			mut sender: Sender<Option<Frame>>) -> Result<()> {
		let notify = Notify::new();

		let heart_beat = match receiver.next().await {
			// We got what we needed.
			Some(Frame::Hello(OpCodeHello {heart_beat})) => {
				let bag = (sender.clone(), Duration::from_millis(heart_beat.into()));

				// Set heart_beat to our heart beat future.
				async {
					let (mut sender, duration) = bag;

					loop {
						// Remove unwrap().
						if let Ok(()) = timeout(duration, notify.notified()).await
							{return Result::Ok(())}
						sender.send(Some(Frame::HeartBeat)).await.unwrap();
					}
				}
			},
			// Expectation failed.
			Some(frame) => return Err(Error::expectation_failed(
				"Frame::Hello(...)", frame)),
			// The channel died, exit gracefully.
			None => return Ok(())
		};

		let login_frame = Frame::Login(OpCodeLogin {
			token: self.client.token.to_owned()
		});
		sender.send(Some(login_frame)).await?;

		let listener = async {
			let result = loop {
				match receiver.next().await {
					Some(Frame::Event(event)) => match event {
						OpCodeEvent::InitState(data) =>
							self.event_handler.on_connect(&self.client, data).await,
						OpCodeEvent::HouseJoin(data) =>
							self.event_handler.on_house_join(&self.client, data).await,
						OpCodeEvent::TypingStart(data) =>
							self.event_handler.on_typing(&self.client, data).await,
						OpCodeEvent::MessageCreate(data) =>
							self.event_handler.on_message(&self.client, data).await
					},
					// The channel died, exit gracefully.
					None => break Result::Ok(()),
					_ => unimplemented!() // Remove unimplemented!().
				}
			};

			notify.notify();
			result
		};

		match join!(heart_beat, listener) {
			(Ok(()), Ok(())) => Ok(()),
			(Err(err), Ok(())) => Err(err),
			(Ok(()), Err(err)) => Err(err),
			(Err(_), Err(err)) => Err(err)
		}
	}
}

#[derive(Debug)]
pub enum Error {
	ExpectationFailed(&'static str, String),
	SocketClose(Option<CloseFrame<'static>>),
	//Serialization(),
	InternalChannel(String)
}

impl Error {
	pub fn expectation_failed<S>(expected: &'static str, got: S) -> Self
			where S: Debug {
		Self::ExpectationFailed(expected, format!("{:?}", got))
	}

	pub fn socket_close(close_data: Option<CloseFrame<'static>>) -> Self {
		Self::SocketClose(close_data)
	}

	pub fn send_error<T>(error: SendError<T>) -> Self
			where T: Debug {
		Self::InternalChannel(format!("{:?}", error))
	}
}

impl<T> From<SendError<T>> for Error
		where T: Debug {
	fn from(error: SendError<T>) -> Self {
		Self::send_error(error)
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
