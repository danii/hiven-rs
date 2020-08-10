#![allow(dead_code)] // File will be worked on later...

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
