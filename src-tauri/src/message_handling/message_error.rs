use core::fmt;

use tokio::io;

use crate::torrent_management::message::Message;

// Enum used to manage possible message errors such as Unknown or Unhandled messages.
#[derive(Debug, PartialEq)]
pub enum MessageError {
    UnknownMessage,
    UnhandledMessage(Message),
    FileIOError,
    ConversionError(String),
    HandshakeError(String),
    NonUnicodePath,
    MismatchedIndex,
    InvalidResponse,
    IOError(String),
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessageError::UnknownMessage => write!(f, "Unknown message"),
            MessageError::UnhandledMessage(msg) => write!(f, "Unhandled message: {:?}", msg),
            MessageError::FileIOError => write!(f, "File IO error"),
            MessageError::NonUnicodePath => write!(f, "Non Unicode path"),
            MessageError::ConversionError(e) => write!(f, "Conversion error: {}", e),
            MessageError::HandshakeError(e) => write!(f, "Handshake error: {}", e),
            MessageError::MismatchedIndex => write!(f, "Mismatched index"),
            MessageError::InvalidResponse => write!(f, "Invalid response"),
            MessageError::IOError(_) => write!(f, "IO error"),
        }
    }
}

impl From<io::Error> for MessageError {
    fn from(error: io::Error) -> Self {
        MessageError::IOError(format!("IO Error: {}", error))
    }
}

impl std::error::Error for MessageError {}
