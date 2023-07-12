use byteorder::{BigEndian, ReadBytesExt};
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::{io::AsyncReadExt, sync::RwLock};

use super::{peer_handshake::initiate_handshake, peers};
use crate::torrent_management::message;
use crate::{
    message_handling::{message_error::MessageError, message_handling::message_handler},
    parsing::parser::torrent_metadata::TorrentMetadata,
};

// Initiates connections to each peer in the given peer list,
// and attempts to start downloading metadata from them.
pub async fn connect_to_peers(
    peers: &Arc<Vec<peers::Peer>>,
    metadata: Arc<RwLock<TorrentMetadata>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let tasks: Vec<_> = peers
        .iter()
        .map(|peer| {
            let metadata = Arc::clone(&metadata);
            let peer = peer.clone(); // Clone the peer to move ownership into the async block.
            spawn_peer_handling_task(peer.clone(), metadata)
        })
        .collect();

    let _ = futures::future::join_all(tasks).await;
    Ok(())
}

async fn spawn_peer_handling_task(peer: peers::Peer, metadata: Arc<RwLock<TorrentMetadata>>) {
    let handshake_metadata = metadata.clone();
    let mut handshake_metadata_guard = handshake_metadata.write().await;
    let mut stream = match initiate_handshake(&peer, &handshake_metadata_guard).await {
        Ok(stream) => {
            println!("TcpStream connected!");
            stream
        }
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };

    let bitfield = match receive_message(&mut stream).await {
        Ok(m) => m,
        Err(e) => {
            println!("Error: {:?}", e);
            return;
        }
    };

    match message_handler(bitfield, &mut *handshake_metadata_guard, 0).await {
        Ok(()) => {
            println!("Message handled successfully");
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    };
}

// Handles the reception of any message from the peer
async fn receive_message(stream: &mut TcpStream) -> Result<message::Message, MessageError> {
    let message_size = bytes_to_u32(&read_n(stream, 4).await?)?;

    if message_size > 0 {
        let message = read_n(stream, message_size).await?;
        message::identify_message(message[0], &message[1..])
    } else {
        // If message size is zero, it's a keep-alive message
        Ok(message::Message::KeepAlive)
    }
}

fn bytes_to_u32(bytes: &[u8]) -> Result<u32, MessageError> {
    let mut rdr = std::io::Cursor::new(bytes);
    ReadBytesExt::read_u32::<BigEndian>(&mut rdr)
        .map_err(|e| MessageError::ConversionError(e.to_string()))
}

pub async fn read_n(stream: &mut TcpStream, nbytes: u32) -> Result<Vec<u8>, MessageError> {
    let mut buffer = vec![0; nbytes as usize];
    match stream.read_exact(&mut buffer).await {
        Ok(_) => Ok(buffer),
        Err(e) => Err(MessageError::ConversionError(e.to_string())),
    }
}
