use serde_bencode::ser::to_bytes;
use serde_bencode::value::Value;
use sha1::{Digest, Sha1};

pub fn compute_sha1_hash(input: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

pub fn compute_info_hash(info: &Value) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match info {
        Value::Dict(dict) => {
            let mut hasher = Sha1::new();
            let key = b"info".to_vec();
            let info_bencode = dict.get(&key).ok_or("info not found in dictionary")?;

            let info_bytes = to_bytes(info_bencode)
                .map_err(|e| format!("Failed to serialize info bencode: {}", e))?;

            hasher.update(info_bytes);
            Ok(hasher.finalize().to_vec())
        }
        _ => Err("info must be a dict".into()),
    }
}
