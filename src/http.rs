use reqwest::Method;
use serde::{Deserialize, Serialize};

pub struct RequestInfo {
	pub path: PathInfo,
	pub token: String,
	pub body: RequestBodyInfo
}

pub enum PathInfo {
	MessageSend {
		channel_id: u64
	},
	MessageEditDelete {
		channel_id: u64,
		message_id: u64
	},
	TypingTrigger {
		channel_id: u64
	}
}

impl PathInfo {
	pub fn path(&self) -> String {
		match self {
			Self::MessageSend {channel_id} =>
				format!("/rooms/{}/messages", channel_id),
			Self::MessageEditDelete {channel_id, message_id, ..} =>
				format!("/rooms/{}/messages/{}", channel_id, message_id),
			Self::TypingTrigger {channel_id} =>
				format!("/rooms/{}/typing", channel_id)
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RequestBodyInfo {
	MessageSend {
		content: String
	},
	MessageDelete,
	TypingTrigger {}
}

impl RequestBodyInfo {
	pub fn method(&self) -> Method {
		match self {
			Self::MessageSend {..} | Self::TypingTrigger {} => Method::POST,
			Self::MessageDelete => Method::DELETE
		}
	}
}

/*pub enum RequestInfo {
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
	pub fn get_path(&self) -> String {
		match self {
			Self::MessageSend {channel_id, ..} =>
				format!("/rooms/{}/messages", channel_id),
			Self::MessageEdit {channel_id, message_id, ..} |
			Self::MessageDelete {channel_id, message_id, ..} =>
				format!("/rooms/{}/messages/{}", channel_id, message_id),
		}
	}

	pub fn get_method(&self) -> &'static str {
		match self {
			Self::MessageSend {..} => "POST",
			Self::MessageEdit {..} => "PATCH",
			Self::MessageDelete {..} => "DELETE"
		}
	}
}*/
