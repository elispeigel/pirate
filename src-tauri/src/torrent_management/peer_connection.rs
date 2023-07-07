use byteorder::{BigEndian, ReadBytesExt};
use std::error::Error;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use super::peers;
use crate::parsing::parser;
use crate::torrent_management::message::message;

// Struct representing the handshake process with a specific peer.
struct Handshake {
    pstr: String,
    info_hash: Vec<u8>, //pass info metadata to extract these fields - string may not be correct. Also pstr should maybe be initialized with a constant..?
    peer_id: String,
}

impl Handshake {
    // Converts Handshake instance into a byte vector representation
    // The vector is used to send handshake messages over the network
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.push(self.pstr.len() as u8); // length of protocol id
        bytes.append(&mut self.pstr.as_bytes().to_vec()); // protocol id
        bytes.append(&mut vec![0u8; 8]); // 8 bytes used to indicate extensions we dont support yet
        bytes.append(&mut self.info_hash.clone()); // 8 bytes used to indicate extensions we dont support yet
        bytes.append(&mut self.peer_id.as_bytes().to_vec()); // 8 bytes used to indicate extensions we dont support yet
        bytes
    }
}

// Initiates connections to each peer in the given peer list,
// and attempts to start downloading metadata from them.
pub async fn connect_to_peers(
    peers: &Arc<Vec<peers::Peer>>,
    metadata: Arc<Mutex<parser::TorrentMetadata>>,
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

async fn spawn_peer_handling_task(
    peer: peers::Peer,
    metadata: Arc<Mutex<parser::TorrentMetadata>>,
) {
    let handshake_metadata = metadata.lock().await.clone();
    let mut stream = match initiate_handshake(&peer, &handshake_metadata).await {
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

    let mut locked_metadata = metadata.lock().await;
    match message::message_handler(bitfield, &mut *locked_metadata, 0).await {
        Ok(()) => {
            println!("Message handled successfully");
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    };
}

// Initiates handshake with given peer, establishing initial connection
async fn initiate_handshake(
    peer: &peers::Peer,
    metadata: &parser::TorrentMetadata,
) -> Result<tokio::net::TcpStream, std::io::Error> {
    let socket = SocketAddr::from(SocketAddrV4::new(peer.ip, peer.port));

    // Use tokio TcpStream to asynchronously establish a connection
    let tcp_stream_result = tokio::net::TcpStream::connect(socket).await;

    match tcp_stream_result {
        Ok(mut tcp_stream) => {
            const DEFAULT_PSTR: &str = "BitTorrent protocol";
            let handshake = Handshake {
                pstr: DEFAULT_PSTR.to_string(),
                info_hash: metadata.info_hash.to_owned(),
                peer_id: "plenty-of-fluid00001".to_string(),
            };
            let handshake_bytes = handshake.to_bytes().clone();
            AsyncWriteExt::write_all(&mut tcp_stream, handshake_bytes.as_slice()).await?;
            println!("Awaiting response");
            match receive_handshake(&mut tcp_stream, metadata.info_hash.to_owned()).await {
                Ok(_) => Ok(tcp_stream),
                Err(e) => Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    e,
                )),
            }
        }
        // Handles case when TCP stream creation fails
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Couldn't connect to the peer",
        )),
    }
}

// Received handshake from the peer
async fn receive_handshake(
    stream: &mut TcpStream,
    our_info_hash: Vec<u8>,
) -> Result<(), message::MessageError> {
    // Reads different portions of the handshake message
    let pstrlen = read_n(stream, 1).await?;
    read_n(stream, pstrlen[0] as u32).await?; // ignore pstr
    read_n(stream, 8).await?; // ignore reserved
    let info_hash = read_n(stream, 20).await?;
    let _peer_id = read_n(stream, 20).await?;

    // Case where received info hash is not same as ours
    if info_hash != our_info_hash {
        Err(message::MessageError::HandshakeError(
            "Invalid info hash".to_string(),
        ))
    } else {
        Ok(())
    }
}

// Handles the reception of any message from the peer

async fn receive_message(
    stream: &mut TcpStream,
) -> Result<message::Message, message::MessageError> {
    let message_size = bytes_to_u32(&read_n(stream, 4).await?)?;

    if message_size > 0 {
        let message = read_n(stream, message_size).await?;
        message::identify_message(message[0], &message[1..])
    } else {
        // If message size is zero, it's a keep-alive message
        Ok(message::Message::KeepAlive)
    }
}

fn bytes_to_u32(bytes: &[u8]) -> Result<u32, message::MessageError> {
    let mut rdr = std::io::Cursor::new(bytes);
    ReadBytesExt::read_u32::<BigEndian>(&mut rdr)
        .map_err(|e| message::MessageError::ConversionError(e.to_string()))
}

async fn read_n(stream: &mut TcpStream, nbytes: u32) -> Result<Vec<u8>, message::MessageError> {
    let mut buffer = vec![0; nbytes as usize];
    match stream.read_exact(&mut buffer).await {
        Ok(_) => Ok(buffer),
        Err(e) => Err(message::MessageError::ConversionError(e.to_string())),
    }
}
