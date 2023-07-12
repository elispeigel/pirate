use std::net::{SocketAddr, SocketAddrV4};

use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::{
    config, message_handling::message_error::MessageError,
    parsing::parser::torrent_metadata::TorrentMetadata,
};

use super::{peer_connection::read_n, peers};

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

// Initiates handshake with given peer, establishing initial connection
pub async fn initiate_handshake(
    peer: &peers::Peer,
    metadata: &TorrentMetadata,
) -> Result<tokio::net::TcpStream, std::io::Error> {
    let socket = SocketAddr::from(SocketAddrV4::new(peer.ip, peer.port));

    // Use tokio TcpStream to asynchronously establish a connection
    let tcp_stream_result = tokio::net::TcpStream::connect(socket).await;

    match tcp_stream_result {
        Ok(mut tcp_stream) => {
            let configuration = config::Config::new();
            let default_pstr = configuration.default_pstr;
            let handshake = Handshake {
                pstr: default_pstr.to_string(),
                info_hash: metadata.info_hash.to_owned(),
                peer_id: metadata.peer_id.to_owned(),
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
) -> Result<(), MessageError> {
    // Reads different portions of the handshake message
    let pstrlen = read_n(stream, 1).await?;
    read_n(stream, pstrlen[0] as u32).await?; // ignore pstr
    read_n(stream, 8).await?; // ignore reserved
    let info_hash = read_n(stream, 20).await?;
    let _peer_id = read_n(stream, 20).await?;

    // Case where received info hash is not same as ours
    if info_hash != our_info_hash {
        Err(MessageError::HandshakeError(
            "Invalid info hash".to_string(),
        ))
    } else {
        Ok(())
    }
}
