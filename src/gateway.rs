use self::super::{data::{ClientSettings, House, Message, User}, util::from_str};
use serde::{
	Deserialize, Serialize,
	de::{Deserializer, Error as DeserializeError, MapAccess, Unexpected, Visitor},
	ser::{SerializeMap, Serializer}
};
use serde_value::Value as UndeserializedAny;
use std::fmt::{Formatter, Result as FMTResult};

#[derive(Debug)]
pub enum Frame {
	Event(OpCodeEvent),
	Hello(OpCodeHello),
	Login(OpCodeLogin),
	HeartBeat
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
			Self::HeartBeat => {
				let mut map = serializer.serialize_map(Some(1))?;
				map.serialize_entry("op", &3)?;
				map.end()
			}
		}
	}
}

struct FrameVisitor;

impl FrameVisitor {
	/// Takes an event string and deserializes map's next value into the
	/// corresponding [OpCodeEvent](enum.OpCodeEvent.html).
	fn deserialize_event<'d, A>(&self, event: &str, map: &mut A)
			-> Result<OpCodeEvent, A::Error> where A: MapAccess<'d> {
		match event {
			// Event deserialization...

			// EventInitState...
			"INIT_STATE" => Ok(OpCodeEvent::InitState(map.next_value()?)),
			// EventHouseJoin...
			"HOUSE_JOIN" => Ok(OpCodeEvent::HouseJoin(map.next_value()?)),
			// EventTypingStart...
			"TYPING_START" => Ok(OpCodeEvent::TypingStart(map.next_value()?)),
			// EventMessageCreate...
			"MESSAGE_CREATE" => Ok(OpCodeEvent::MessageCreate(map.next_value()?)),

			// Invalid event...
			event => Err(DeserializeError::invalid_value(Unexpected::Str(event),
				&"valid event"))
		}
	}
}

impl<'d> Visitor<'d> for FrameVisitor {
	type Value = Frame;

	fn expecting(&self, formatter: &mut Formatter) -> FMTResult {
		write!(formatter, "a hiven.io gateway websocket frame")
	}

	/// Deserializes a serde map structure into a
	/// [Frame](struct.FrameVisitor.html).
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
		const FIELDS: [&str; 4] = ["op", "d", "e", "seq"];

		// Iterate over key values...
		while let Some(key) = map.next_key()? {match key {
			// Ignore sequence, for now...
			"seq" => {map.next_value::<UndeserializedAny>()?;},
			"op" => {
				op_code = Some(map.next_value()?);
				break
			},
			"e" => if event.is_none() {
				event = Some(map.next_value()?)
			} else {
				return Err(DeserializeError::duplicate_field("e"))
			},
			"d" => {
				data = Some(map.next_value()?);
				break
			},

			_ => return Err(A::Error::unknown_field(key, &FIELDS))
		}}

		if let (Some(op_code), None) = (op_code, &mut data) {
			// "op" was found before "d". No need to use UndeserializedAny.
			let mut result: Option<Frame> = None;

			// Iterate over key values...
			while let Some(key) = map.next_key()? {match key {
				// Ignore sequence, for now...
				"seq" => {map.next_value::<UndeserializedAny>()?;},
				"op" => return Err(DeserializeError::duplicate_field("op")),
				"e" => if event.is_none() {event = Some(map.next_value()?)}
					else {return Err(DeserializeError::duplicate_field("e"))},
				"d" => if result.is_none() && data.is_none() {match op_code {
					//// OpCode deserialization...

					// OpCodeEvent...
					0 => if let Some(event) = event {
						let value = self.deserialize_event(event, &mut map)?;
						result = Some(Frame::Event(value))
					} else {
						data = Some(map.next_value()?);
						break
					},
					// OpCodeHello...
					1 => result = Some(Frame::Hello(map.next_value()?)),
					// OpCodeLogin...
					2 => result = Some(Frame::Login(map.next_value()?)),

					// Operation codes that don't have data...
					3 => return Err(DeserializeError::unknown_field("d", &[])),
					// Unknown operation code...
					_ => return Err(DeserializeError::invalid_value(
						Unexpected::Unsigned(op_code.into()), &"valid opcode"))
				}} else {
					return Err(DeserializeError::duplicate_field("d"))
				},

				_ => return Err(A::Error::unknown_field(key, &[]))
			}}

			if result.is_none() && data.is_some() {
				// "op" was 0 and "d" was found before "e".
				// We must use UndeserializedAny.
				let event = event.unwrap();
				let result = self.deserialize_event(event, &mut map)?;

				Ok(Frame::Event(result))
			} else {
				// "op" was not 0.
				Ok(match result {
					Some(result) => result,
					// Operation codes that don't have data...
					None => match op_code {
						//// OpCode deserialization...

						// OpCodeHeartBeat...
						3 => Frame::HeartBeat,

						// Operation codes that have data...
						0 | 1 | 2 => return Err(DeserializeError::missing_field("d")),
						// Unknown operation code...
						_ => return Err(DeserializeError::invalid_value(
							Unexpected::Unsigned(op_code.into()), &"valid opcode"))
					}
				})
			}
		} else if let (None, Some(data)) = (op_code, &mut data) {
			// "d" was found before "op". We must use UndeserializedAny.
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
					//// OpCode deserialization...

					// OpCodeEvent...
					0 => if let Some(event) = event {
						let value = self.deserialize_event(event, &mut map)?;
						result = Some(Frame::Event(value))
					} else {
						op_zero = true
					},
					// OpCodeHello...
					1 => result = Some(Frame::Hello(data_into()?)),

					// Operation codes that don't have data...
					3 => return Err(DeserializeError::unknown_field("d", &[])),
					// Unknown operation code...
					op_code => return Err(DeserializeError::invalid_value(
						Unexpected::Unsigned(op_code.into()), &"valid opcode"))
				}} else {
					return Err(DeserializeError::duplicate_field("op"))
				},
				"e" => if op_zero {
					let value = self.deserialize_event(map.next_value()?, &mut map)?;
					result = Some(Frame::Event(value))
				} else if event.is_none() {
					event = Some(map.next_value()?)
				} else {
					return Err(DeserializeError::duplicate_field("e"))
				},
				"d" => return Err(DeserializeError::duplicate_field("d")),

				_ => return Err(A::Error::unknown_field(key, &[]))
			}}

			result.ok_or_else(|| DeserializeError::missing_field(
				if op_zero {"e"} else {"op"}))
		} else {
			Err(DeserializeError::missing_field("op"))
		}
	}
}

// Automatically serialized and deserialized by Frame.
#[derive(Debug)]
pub enum OpCodeEvent {
	InitState(EventInitState),
	HouseJoin(House),
	TypingStart(EventTypingStart),
	MessageCreate(Message)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpCodeHello {
	#[serde(rename = "hbt_int")]
	pub heart_beat: u16
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpCodeLogin {
	pub token: String
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventInitState {
	pub user: User,
	pub settings: ClientSettings
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventTypingStart {
	#[serde(deserialize_with = "from_str")]
	pub room_id: u64,
	#[serde(rename = "author_id", deserialize_with = "from_str")]
	pub user_id: u64
}

#[cfg(test)]
mod tests {
	#[test]
	fn serilization_test() {
		
	}
}
