use crate::parsing::parser;
use core::fmt;
use std::fs::OpenOptions;
use std::io::{self, Seek, SeekFrom, Write};

// Struct that represents the payload of a message.
// Contains an ID, the actual message, and any payload that comes with the message.
#[derive(Debug)]
pub struct MessagePayload {
    pub message_id: u8,
    pub message: Message,
    pub payload: Vec<u8>,
}

// Enum used to manage possible message errors such as Unknown or Unhandled messages.
#[derive(Debug, PartialEq)]
pub enum MessageError {
    UnknownMessage,
    UnhandledMessage(Message),
    FileIOError,
    ConversionError(String),
    HandshakeError(String),
    NonUnicodePath,
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
        }
    }
}

impl std::error::Error for MessageError {}

// Enum representing all potential message types in the defined protocol.
#[derive(Debug, PartialEq)]
pub enum Message {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield(Vec<u8>),
    Request,
    Piece(usize, usize, Vec<u8>),
    Cancel,
    KeepAlive,
}

// Function to identify the type of message according to its id.
// Returns a Message enum instance on success or a MessageError otherwise.
pub fn identify_message(message_id: u8, message_body: &[u8]) -> Result<Message, MessageError> {
    // Match the ID of the incoming message to known types in the protocol
    match message_id {
        0 => Ok(Message::Choke),
        1 => Ok(Message::Unchoke),
        2 => Ok(Message::Interested),
        3 => Ok(Message::NotInterested),
        4 => Ok(Message::Have),
        5 => Ok(Message::Bitfield(message_body.to_vec())),
        6 => Ok(Message::Request),
        7 => {
            if message_body.len() < 8 {
                return Err(MessageError::UnknownMessage);
            }

            let index = if message_body.len() >= 4 {
                Some(usize::from_le_bytes(
                    message_body[0..4]
                        .try_into()
                        .expect("Expected 4 bytes for index"),
                ))
            } else {
                return Err(MessageError::UnknownMessage);
            };

            let begin = if message_body.len() >= 8 {
                Some(usize::from_le_bytes(
                    message_body[4..8]
                        .try_into()
                        .expect("Expected 4 bytes for begin"),
                ))
            } else {
                return Err(MessageError::UnknownMessage);
            };

            let data = message_body[8..].to_vec();

            if let (Some(index), Some(begin)) = (index, begin) {
                Ok(Message::Piece(index, begin, data))
            } else {
                Err(MessageError::UnknownMessage)
            }
        }
        8 => Ok(Message::Cancel),
        _ => Err(MessageError::UnknownMessage),
    }
}

// Handle the message received accordingly and manipulate the TorrentMetadata.
pub async fn message_handler(
    msg: Message,
    metadata: &mut parser::TorrentMetadata,
    piece_index: usize,
) -> Result<(), MessageError> {
    match msg {
        Message::Choke => {
            println!("Choke message received.");
            Ok(())
        }
        Message::Unchoke => {
            println!("Unchoke message received.");
            Ok(())
        }
        Message::Interested => {
            println!("Interested message received.");
            Ok(())
        }
        Message::NotInterested => {
            println!("NotInterested message received.");
            Ok(())
        }
        Message::Have => {
            println!("Have message received.");
            Ok(())
        }
        Message::Bitfield(body) => {
            bitfield_handler(body)?;
            Ok(())
        }
        Message::Request => {
            println!("Request message received.");
            Ok(())
        }
        Message::Piece(index, begin, data) => {
            let piece_size = metadata.info.piece_length as usize;
            let write_result = tokio::task::block_in_place(|| match metadata.file_path.to_str() {
                Some(path_str) => write_to_file(index, begin, piece_size, data, path_str)
                    .map_err(|_| MessageError::FileIOError),
                None => Err(MessageError::NonUnicodePath),
            });

            match write_result {
                Ok(_) => {
                    metadata.pieces_status[piece_index].downloaded = true;
                    if parser::check_all_pieces_downloaded(metadata) {
                        println!("Download complete!");
                    }
                    Ok(())
                }
                Err(_) => Err(MessageError::FileIOError),
            }
        }
        Message::Cancel => {
            println!("Cancel message received.");
            Ok(())
        }
        Message::KeepAlive => {
            println!("Received KeepAlive");
            Ok(())
        }
    }
}

// Writes the received data to a file
fn write_to_file(
    index: usize,
    begin: usize,
    piece_size: usize,
    data: Vec<u8>,
    file_path: &str,
) -> io::Result<()> {
    let position = index * piece_size + begin;

    let mut file = OpenOptions::new().write(true).open(file_path)?;

    file.seek(SeekFrom::Start(position as u64))?;

    file.write_all(&data)?;

    Ok(())
}

fn bitfield_handler(bitfield: Vec<u8>) -> Result<(), MessageError> {
    println!("Bitfield: {:?}", bitfield);
    Ok(())
}
