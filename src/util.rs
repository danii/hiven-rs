use serde::{
	Deserialize,
	de::{Deserializer, Error as DeserializeError, Unexpected}
};

const FROM_STR_ERR: &'static str =
"string value that can be parsed into other values";

pub(crate) fn from_str<'d, T, D>(deserializer: D) -> Result<T, D::Error>
		where T: std::str::FromStr,
			D: Deserializer<'d> {
	let string = <&'d str>::deserialize(deserializer)?;
	T::from_str(&string).map_err(|_| DeserializeError::invalid_value(
		Unexpected::Str(string), &FROM_STR_ERR))
}

pub(crate) fn from_str_opt<'d, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
		where T: std::str::FromStr,
			D: Deserializer<'d> {
	let string = Option::<&'d str>::deserialize(deserializer)?;
	Ok(match string {
		Some(string) => Some(T::from_str(&string).map_err(|_| DeserializeError
			::invalid_value(Unexpected::Str(string), &FROM_STR_ERR))?),
		None => None
	})
}
