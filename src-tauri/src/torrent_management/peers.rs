use crate::{
    config,
    parsing::parser::torrent_metadata::TorrentMetadata,
    tracker::{self, build_tracker_query},
};
use serde_bencode::value::Value;
use std::convert::TryInto;
use std::net::Ipv4Addr;

#[derive(Debug, Clone, PartialEq)]
pub struct Peer {
    pub ip: Ipv4Addr,
    pub port: u16,
}

pub async fn get_peers(metadata: &TorrentMetadata) -> Result<Vec<Peer>, String> {
    let query = build_tracker_query(metadata).await?;

    let response_bytes = match tracker::execute_tracker_query(query).await {
        Ok(data) => data,
        Err(e) => return Err(e),
    };

    let bencoded_response = match serde_bencode::de::from_bytes(&response_bytes) {
        Ok(bencoded_response) => bencoded_response,
        Err(_) => return Err("Failed to decode bencoded data.".into()),
    };

    let response_dict = if let Value::Dict(dict) = bencoded_response {
        dict.clone()
    } else {
        return Err("Response should be a dict!".to_string());
    };

    let peer_list = match response_dict.get(&b"peers".to_vec()) {
        Some(Value::Bytes(s)) => s.clone(),
        _ => return Err("Expected peers to be a ByteString".to_string()),
    };

    unmarshal_peers(&peer_list)
}

pub fn unmarshal_peers(peers: &Vec<u8>) -> Result<Vec<Peer>, String> {
    let configuration = config::Config::new();
    let peer_size = configuration.peer_size;
    let mut unmarshalled_peers: Vec<Peer> = Vec::new();

    if peers.len() as u16 % peer_size != 0 {
        return Err("Received malformed peers".to_string());
    }
    let peer_chunks: Vec<&[u8]> = peers.chunks(peer_size as usize).collect();

    for chunk in peer_chunks {
        // Split the chunk into IP and port parts
        let (ip_part, port_part) = chunk.split_at(4);

        // Convert port bytes into u16
        let port = match port_part.try_into() {
            Ok(array) => u16::from_be_bytes(array),
            Err(_) => return Err(format!("Invalid port value in chunk: {:?}", chunk)),
        };

        // Create a new Peer object and push it to the vector
        unmarshalled_peers.push(Peer {
            ip: Ipv4Addr::new(ip_part[0], ip_part[1], ip_part[2], ip_part[3]),
            port,
        });
    }

    Ok(unmarshalled_peers)
}
