use serde::{
	Deserialize, Serialize,
	de::{Deserializer, Error as DeserializeError, Unexpected}
};

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
	username: String,
	name: String,
	icon: Option<String>,
	header: Option<String>,
	#[serde(deserialize_with = "from_str")]
	id: u64
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientSettings {
	theme: Option<Theme>,
	#[serde(rename = "enable_desktop_notifications")]
	desktop_notifications: Option<bool>
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Theme {
	#[serde(rename = "dark")]
	Dark
	//???????
}

fn from_str<'d, T, D>(deserializer: D) -> Result<T, D::Error>
		where T: std::str::FromStr,
			D: Deserializer<'d> {
	let string = <&'d str>::deserialize(deserializer)?;
	T::from_str(&string).map_err(|_| DeserializeError::invalid_value(
		Unexpected::Str(string), &"string value that can be parsed into other values"))
}
