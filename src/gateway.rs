use derive_into_owned::IntoOwned;
use serde::{
	Deserialize, Serialize,
	de::{Deserializer, Error as DeserializeError, MapAccess, Unexpected, Visitor},
	ser::{SerializeMap, Serializer}
};
use serde_value::{Unexpected as DeserializableUnexpected, Value as UndeserializedAny};
use std::{borrow::Cow, fmt::{Formatter, Result as FMTResult}};

#[derive(Debug, IntoOwned)]
pub enum Frame {
	Hello(HelloOpCode),
	Login(LoginOpCode),
	HeartBeatAck
}

impl<'d> Deserialize<'d> for Frame {
	fn deserialize<D>(deserialzer: D) -> Result<Self, D::Error>
			where D: Deserializer<'d> {
		deserialzer.deserialize_map(FrameVisitor)
	}
}

impl Serialize for Frame {
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
			},
			Self::HeartBeatAck => {
				let mut map = serializer.serialize_map(Some(1))?;
				map.serialize_entry("op", &3)?;
				map.end()
			}
		}
	}
}

struct FrameVisitor;

impl<'d> Visitor<'d> for FrameVisitor {
	type Value = Frame;

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
					// HelloOpCode...
					1 => result = Some(Frame::Hello(map.next_value()?)),
					// LoginOpCode...
					2 => result = Some(Frame::Login(map.next_value()?)),

					// Operation codes that don't have data...
					3 => Err(DeserializeError::unknown_field("d", &[]))?,
					// Unknown operation code...
					_ => Err(DeserializeError::invalid_value(
						Unexpected::Unsigned(op_code.into()), &"valid opcode"))?
				}} else {Err(DeserializeError::duplicate_field("d"))?},
				_ => Err(A::Error::unknown_field(key, &[]))?
			}}

			Ok(match result {
				Some(result) => result,
				// Operation codes that don't have data...
				None => match op_code {
					// HeartBeatAckOpCode...
					3 => Frame::HeartBeatAck,

					// Operation codes that have data...
					1 | 2 => Err(DeserializeError::missing_field("d"))?,
					// Unknown operation code...
					_ => Err(DeserializeError::invalid_value(
						Unexpected::Unsigned(op_code.into()), &"valid opcode"))?
				}
			})
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
					// HelloOpCode...
					1 => result = Some(Frame::Hello(data_into()?)),

					// Operation codes that don't have data...
					3 => Err(DeserializeError::unknown_field("d", &[]))?,
					// Unknown operation code...
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

#[derive(Debug, Deserialize, IntoOwned, Serialize)]
pub struct HelloOpCode {
	#[serde(rename = "hbt_int")]
	pub heart_beat: u16
}

#[derive(Debug, Deserialize, IntoOwned, Serialize)]
pub struct LoginOpCode {
	pub token: String
}

#[cfg(test)]
mod tests {
	#[test]
	fn serilization_test() {
		
	}
}
