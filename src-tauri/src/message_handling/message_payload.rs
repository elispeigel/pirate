use crate::torrent_management::message::Message;

#[derive(Debug)]
pub struct MessagePayload {
    pub message_id: u8,
    pub message: Message,
    pub payload: Vec<u8>,
}
