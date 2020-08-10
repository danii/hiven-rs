use self::super::data::{ClientSettings, User};
use derive_into_owned::IntoOwned;
use serde::{
	Deserialize, Serialize,
	de::{Deserializer, Error as DeserializeError, MapAccess, Unexpected, Visitor},
	ser::{SerializeMap, Serializer}
};
use serde_value::Value as UndeserializedAny;
use std::fmt::{Formatter, Result as FMTResult};

#[derive(Debug, IntoOwned)]
pub enum Frame {
	Event(EventOpCode),
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
			Self::Event(_) => {
				unimplemented!("C")
			},
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

	// This function is quite a bit of spaghetti code, but it should
	// deserialize all hiven opcodes. To aid in reading, I've plastered comments
	// everywhere, which should (hopefully) make it easier to understand.
	// Yes, I would have used derive, except derive can't deserialize a data
	// structure like this.
	fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where A: MapAccess<'d> {
		let mut event: Option<&'d str> = None;
		let mut op_code: Option<u8> = None;
		let mut data: Option<UndeserializedAny> = None;
		const FIELDS: [&'static str; 4] = ["op", "d", "e", "seq"];

		// Iterate over key values...
		while let Some(key) = map.next_key()? {match key {
			// Ignore sequence, for now...
			"seq" => {map.next_value::<UndeserializedAny>()?;},
			"op" => {
				op_code = Some(map.next_value()?);
				break
			},
			"e" => if let None = event {event = Some(map.next_value()?)}
				else {Err(DeserializeError::duplicate_field("e"))?},
			"d" => {
				data = Some(map.next_value()?);
				break
			},
			_ => Err(A::Error::unknown_field(key, &FIELDS))?
		}}

		if op_code.is_some() && data.is_none() {
			// "op" was found before "d". No need to use UndeserializedAny.
			let op_code = op_code.unwrap();
			let mut result: Option<Frame> = None;

			// Iterate over key values...
			while let Some(key) = map.next_key()? {match key {
				// Ignore sequence, for now...
				"seq" => {map.next_value::<UndeserializedAny>()?;},
				"op" => Err(DeserializeError::duplicate_field("op"))?,
				"e" => if let None = event {event = Some(map.next_value()?)}
					else {Err(DeserializeError::duplicate_field("e"))?},
				"d" => if result.is_none() && data.is_none() {match op_code {
					// OpCode deserialization...

					// EventOpCode...
					0 => if let None = event {
						data = Some(map.next_value()?);
						break
					} else {match event.unwrap() {
						// "op" was 0 and "e" was found before "d".
						// No need to use UndeserializedAny.
						// Event deserialization...

						// InitState...
						"INIT_STATE" => result =
							Some(Frame::Event(EventOpCode::InitState(map.next_value()?))),

						// Invalid event...
						event @ _ => Err(DeserializeError::invalid_value(
							Unexpected::Str(event), &"valid event"))?
					}},
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

			if result.is_none() && data.is_some() {
				// "op" was 0 and "d" was found before "e".
				// We must use UndeserializedAny.
				let event = event.unwrap();
				let result: EventOpCode = match event {
					// Event deserialization...

					// InitState...
					"INIT_STATE" => EventOpCode::InitState(map.next_value()?),

					// Invalid event...
					event @ _ => Err(DeserializeError::invalid_value(
						Unexpected::Str(event), &"valid event"))?
				};

				Ok(Frame::Event(result))
			} else {
				// "op" was not 0.
				Ok(match result {
					Some(result) => result,
					// Operation codes that don't have data...
					None => match op_code {
						// OpCode deserialization...

						// HeartBeatAckOpCode...
						3 => Frame::HeartBeatAck,

						// Operation codes that have data...
						0 | 1 | 2 => Err(DeserializeError::missing_field("d"))?,
						// Unknown operation code...
						_ => Err(DeserializeError::invalid_value(
							Unexpected::Unsigned(op_code.into()), &"valid opcode"))?
					}
				})
			}
		} else if op_code.is_none() && data.is_some() {
			// "d" was found before "op". We must use UndeserializedAny.
			let data = data.unwrap();
			let mut result: Option<Frame> = None;
			let mut op_zero = false; // If set to true, construct result in "e" arm.
			// .clone may be removed when https://github.com/rust-lang/rfcs/pull/2593
			// is pulled. (Enum variant types may give us a way to guard the moving of
			// data statically.)
			let data_into = || data.clone().deserialize_into()
				.map_err(|err| err.into_error());

			// Iterate over key values...
			while let Some(key) = map.next_key()? {match key {
				// Ignore sequence, for now...
				"seq" => {map.next_value::<UndeserializedAny>()?;},
				"op" => if result.is_none() && !op_zero {match map.next_value::<u8>()? {
					// OpCode deserialization...

					// EventOpCode...
					0 => if let Some(event) = event {result = Some(Frame::Event(
						match event {
							// Event deserialization...

							// InitState...
							"INIT_STATE" => EventOpCode::InitState(map.next_value()?),

							// Invalid event...
							event @ _ => Err(DeserializeError::invalid_value(
								Unexpected::Str(event), &"valid event"))?
						}
					))} else {op_zero = true},
					// HelloOpCode...
					1 => result = Some(Frame::Hello(data_into()?)),

					// Operation codes that don't have data...
					3 => Err(DeserializeError::unknown_field("d", &[]))?,
					// Unknown operation code...
					op_code @ _ => Err(DeserializeError::invalid_value(
						Unexpected::Unsigned(op_code.into()), &"valid opcode"))?
				}} else {Err(DeserializeError::duplicate_field("op"))?},
				"e" => if op_zero {result = Some(Frame::Event(match map.next_value()? {
					// Event deserialization...

					// InitState...
					"INIT_STATE" => EventOpCode::InitState(map.next_value()?),

					// Invalid event...
					event @ _ => Err(DeserializeError::invalid_value(
						Unexpected::Str(event), &"valid event"))?
				}))} else if let None = event {event = Some(map.next_value()?)}
				else {Err(DeserializeError::duplicate_field("e"))?},
				"d" => Err(DeserializeError::duplicate_field("d"))?,
				_ => Err(A::Error::unknown_field(key, &[]))?
			}}

			result.ok_or(DeserializeError::missing_field(
				if op_zero {"e"} else {"op"}))
		} else {
			Err(DeserializeError::missing_field("op"))
		}
	}
}

// Automatically serialized and deserialized by Frame.
#[derive(Debug, IntoOwned)]
pub enum EventOpCode {
	InitState(InitStateEvent)
}

#[derive(Debug, Deserialize, IntoOwned, Serialize)]
pub struct InitStateEvent {
	user: User,
	settings: ClientSettings
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
