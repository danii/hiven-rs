use self::super::util::{from_str, from_str_opt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct House {
	pub name: String,
	pub icon: Option<String>,
	pub members: Vec<Member>,
	pub rooms: Vec<Room>,
	#[serde(deserialize_with = "from_str")]
	pub id: u64,
	#[serde(deserialize_with = "from_str")]
	pub owner_id: u64
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Member {
	pub user: User,
	pub presence: Presence
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Room {
	pub name: String,
	pub description: Option<String>,
	//pub emoji:
	pub position: usize,
	#[serde(default)]
	#[serde(deserialize_with = "from_str_opt")]
	pub last_message_id: Option<u64>,
	#[serde(deserialize_with = "from_str")]
	pub id: u64
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
	pub content: String,
	#[serde(deserialize_with = "from_str")]
	pub room_id: u64,
	#[serde(deserialize_with = "from_str")]
	pub author_id: u64
}

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
	pub username: String,
	pub name: String,
	pub icon: Option<String>,
	pub header: Option<String>,
	#[serde(deserialize_with = "from_str")]
	pub id: u64
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientSettings {
	pub theme: Option<Theme>,
	#[serde(rename = "enable_desktop_notifications")]
	pub desktop_notifications: Option<bool>
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Presence {
	#[serde(rename = "offline")]
	Offline,
	#[serde(rename = "online")]
	Online
	//?????
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Theme {
	#[serde(rename = "dark")]
	Dark
	//???????
}
