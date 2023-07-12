use crate::message_handling::message_error::MessageError;
use serde::Serialize;
use serde_bencode::to_bytes;
// Struct that represents the payload of a message.
// Contains an ID, the actual message, and any payload that comes with the message.

// Enum representing all potential message types in the defined protocol.
#[derive(Debug, PartialEq, Serialize)]
pub enum Message {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield(Vec<u8>),
    Request(u32),
    Piece(usize, usize, Vec<u8>),
    Cancel,
    KeepAlive,
}

impl Message {
    pub fn encode(&self) -> Vec<u8> {
        to_bytes(self).expect("Failed to encode message")
    }
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
        6 => {
            if let Some(index) = message_body.get(0) {
                Ok(Message::Request(*index as u32))
            } else {
                Err(MessageError::UnknownMessage)
            }
        }
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
