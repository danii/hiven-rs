use serde::{
	Deserialize, Serialize,
	de::{Deserializer, Error as DeserializeError, MapAccess, Unexpected, Visitor},
	ser::{SerializeMap, Serializer}
};
use serde_value::Value as UndeserializedAny;
use std::fmt::{Formatter, Result as FMTResult};

#[derive(Debug)]
pub enum Frame<'d> {
	Hello(HelloOpCode),
	Login(LoginOpCode<'d>)
}

impl<'d> Deserialize<'d> for Frame<'d> {
	fn deserialize<D>(deserialzer: D) -> Result<Self, D::Error>
			where D: Deserializer<'d> {
		deserialzer.deserialize_map(FrameVisitor)
	}
}

impl<'d> Serialize for Frame<'d> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where S: Serializer {
		match self {
			Self::Hello(op_code) => {
				let mut map = serializer.serialize_map(Some(2))?;
				map.serialize_entry("op", &1)?;
				map.serialize_entry("d", op_code)?;
				map.end()
			},
			Self::Login(op_code) => {
				let mut map = serializer.serialize_map(Some(2))?;
				map.serialize_entry("op", &2)?;
				map.serialize_entry("d", op_code)?;
				map.end()
			}
		}
	}
}

struct FrameVisitor;

impl<'d> Visitor<'d> for FrameVisitor {
	type Value = Frame<'d>;

	fn expecting(&self, formatter: &mut Formatter) -> FMTResult {
		write!(formatter, "a hiven.io gateway websocket frame")
	}

	fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where A: MapAccess<'d> {
		let mut op_code: Option<u8> = None;
		let mut data: Option<UndeserializedAny> = None;

		while let Some(key) = map.next_key()? {match key {
			"op" => {
				op_code = Some(map.next_value()?);
				break
			},
			"d" => {
				data = Some(map.next_value()?);
				break
			}
			_ => Err(A::Error::unknown_field(key, &[]))?
		}}

		if op_code.is_some() && data.is_none() {
			// "op" was found before "d". No need to use UndeserializedAny.
			let op_code = op_code.unwrap();
			let mut result: Option<Frame> = None;

			while let Some(key) = map.next_key()? {match key {
				"op" => Err(DeserializeError::duplicate_field("op"))?,
				"d" => if let None = result {match op_code {
					1 => result = Some(Frame::Hello(map.next_value()?)),
					_ => Err(DeserializeError::invalid_value(
						Unexpected::Unsigned(op_code.into()), &"valid opcode"))?
				}} else {Err(DeserializeError::duplicate_field("d"))?},
				_ => Err(A::Error::unknown_field(key, &[]))?
			}}

			result.ok_or(DeserializeError::missing_field("d"))
		} else if op_code.is_none() && data.is_some() {
			// "d" was found before "op". We must use UndeserializedAny.
			let data = data.unwrap();
			let mut result: Option<Frame> = None;
			// .clone may be removed when https://github.com/rust-lang/rfcs/pull/2593
			// is pulled. (Enum variant types may give us a way to guard the moving of
			// data statically.)
			let data_into = || data.clone().deserialize_into().map_err(|err| err.into_error());

			while let Some(key) = map.next_key()? {match key {
				"op" => if let None = result {match map.next_value::<u8>()? {
					1 => result = Some(Frame::Hello(data_into()?)),
					op_code @ _ => Err(DeserializeError::invalid_value(
						Unexpected::Unsigned(op_code.into()), &"valid opcode"))?
				}} else {Err(DeserializeError::duplicate_field("op"))?},
				"d" => Err(DeserializeError::duplicate_field("d"))?,
				_ => Err(A::Error::unknown_field(key, &[]))?
			}}

			result.ok_or(DeserializeError::missing_field("op"))
		} else {
			Err(DeserializeError::missing_field("op"))
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HelloOpCode {
	#[serde(rename = "hbt_int")]
	pub heart_beat: u16
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginOpCode<'d> {
	#[serde(rename = "hbt_int")]
	token: &'d str
}
