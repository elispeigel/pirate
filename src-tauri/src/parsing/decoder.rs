use super::parser::ParseError;
use serde_bencode::ser::to_bytes;
use serde_bencode::value::Value;
use std::collections::HashMap;

pub fn get_number_from_bencode(
    dict: &HashMap<Vec<u8>, Value>,
    field: &str,
) -> Result<i64, ParseError> {
    match get_field(dict, field) {
        Ok(Value::Int(n)) => Ok(n),
        Ok(_) => Err(ParseError::ParseError(format!(
            "Field {} is not a Number",
            field
        ))),
        Err(e) => Err(e),
    }
}

pub fn get_string_from_bencode(
    dict: &HashMap<Vec<u8>, Value>,
    field: &str,
) -> Result<String, ParseError> {
    match get_field(dict, field) {
        Ok(Value::Bytes(b)) => String::from_utf8(b.to_vec()).map_err(ParseError::from),
        Ok(_) => Err(ParseError::ParseError(format!(
            "Field {} is not a String",
            field
        ))),
        Err(e) => Err(e),
    }
}

pub fn get_field(dict: &HashMap<Vec<u8>, Value>, field: &str) -> Result<Value, ParseError> {
    match dict.get(&field.as_bytes().to_vec()) {
        Some(a) => Ok(a.to_owned()),
        None => Err(ParseError::ParseError("Could not find field".to_string())),
    }
}

pub fn get_field_as_dict(
    dict: &HashMap<Vec<u8>, Value>,
    field: &str,
) -> Result<HashMap<Vec<u8>, Value>, ParseError> {
    match get_field(dict, field) {
        Ok(Value::Dict(dict)) => Ok(dict),
        Ok(_) => Err(ParseError::ParseError("Field is not a Dict".into())),
        Err(e) => Err(e),
    }
}

pub fn get_field_as_bencoded_bytes(
    dict: &HashMap<Vec<u8>, Value>,
    field: &str,
) -> Result<Vec<u8>, ParseError> {
    match dict.get(&field.as_bytes().to_vec()) {
        Some(raw_field) => match to_bytes(raw_field) {
            Ok(bytes) => Ok(bytes),
            Err(_) => Err(ParseError::ParseError(format!(
                "Failed to convert field {} to bytes",
                field
            ))),
        },
        None => Err(ParseError::ParseError(format!(
            "Could not find field: {}",
            field
        ))),
    }
}

pub fn get_field_as_20_byte_array(
    dict: &HashMap<Vec<u8>, Value>,
    field: &str,
) -> Result<[u8; 20], ParseError> {
    let bytes = get_field_as_bencoded_bytes(dict, field)?;

    bytes
        .chunks_exact(20)
        .next()
        .ok_or_else(|| {
            ParseError::ParseError(format!(
                "Failed to convert field {} to a 20-byte array",
                field
            ))
        })
        .map(|chunk| {
            let mut array = [0u8; 20];
            array.copy_from_slice(chunk);
            array
        })
}
