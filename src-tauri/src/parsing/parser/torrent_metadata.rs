use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct TorrentMetadata {
    pub info: TorrentMetadataInfo,
    pub info_hash: Vec<u8>,
    pub announce: String,
    pub file_path: PathBuf,
    pub peer_id: String,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct TorrentMetadataInfo {
    pub pieces: Vec<u8>,
    pub piece_length: i64,
    pub length: i64,
    pub name: String,
}
