use serde_bencode::ser::to_bytes;
use serde_bencode::value::Value;
use sha1::{Digest, Sha1};

pub fn compute_info_hash(info: &Value) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut hasher = Sha1::new();
    let info_bytes =
        to_bytes(info).map_err(|e| format!("Failed to serialize info bencode: {}", e))?;
    hasher.update(info_bytes);
    Ok(hasher.finalize().to_vec())
}
