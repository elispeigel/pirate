use serde::Deserialize;
use serde_bencode::de;
use std::path::PathBuf;

#[derive(PartialEq, Debug, Clone, Deserialize)]
pub struct PieceStatus {
    pub downloaded: bool,
}

#[derive(PartialEq, Debug, Clone, Deserialize)]
pub struct TorrentMetadata {
    pub info: TorrentMetadataInfo,
    pub info_hash: Vec<u8>,
    pub announce: String,
    pub pieces_status: Vec<PieceStatus>,
    pub file_path: PathBuf,
}

#[derive(PartialEq, Debug, Clone, Deserialize)]
pub struct TorrentMetadataInfo {
    pub pieces: Vec<u8>,
    pub piece_length: i64,
    pub length: i64,
    pub name: String,
}

pub fn check_all_pieces_downloaded(metadata: &mut TorrentMetadata) -> bool {
    metadata.pieces_status.iter().all(|piece| piece.downloaded)
}

#[derive(Debug)]
pub enum ParseError {
    IoError(std::io::Error),
    ParseError(String),
    Utf8Error(std::string::FromUtf8Error),
    BencodeError(serde_bencode::Error),
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> ParseError {
        ParseError::IoError(err)
    }
}

impl From<std::string::FromUtf8Error> for ParseError {
    fn from(err: std::string::FromUtf8Error) -> ParseError {
        ParseError::Utf8Error(err)
    }
}

impl From<serde_bencode::Error> for ParseError {
    fn from(err: serde_bencode::Error) -> ParseError {
        ParseError::BencodeError(err)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::IoError(err) => write!(f, "IO error: {}", err),
            ParseError::ParseError(err) => write!(f, "Parse error: {}", err),
            ParseError::Utf8Error(err) => write!(f, "UTF-8 error: {}", err),
            ParseError::BencodeError(err) => write!(f, "Bencode error: {:?}", err),
        }
    }
}

pub fn parse_bencoded_torrent(bencoded_metadata: Vec<u8>) -> Result<TorrentMetadata, ParseError> {
    let bencode: Result<TorrentMetadata, _> = de::from_bytes(&bencoded_metadata);

    match bencode {
        Ok(torrent_metadata) => Ok(torrent_metadata),
        _ => {
            return Err(ParseError::ParseError(
                "Top level bencode should be a dict".into(),
            ))
        }
    }
}
