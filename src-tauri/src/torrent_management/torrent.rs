use crate::{
    message_handling::message_error::MessageError,
    parsing::parser::torrent_metadata::TorrentMetadata, peers::Peer,
};
use bitvec::prelude::BitVec;
use bitvec::prelude::Lsb0;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use sha1::{Digest, Sha1};
use std::{collections::HashMap, net::Ipv4Addr, sync::Arc};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt, Result},
    net::TcpStream,
    sync::{Mutex, RwLock},
};

use super::{
    file_io::save_piece_to_disk,
    message::{self, Message},
    torrent_status::TorrentStatus,
};

pub struct Torrent {
    info_hash: [u8; 20],
    current_downloaded: AtomicU64,
    current_uploaded: AtomicU64,
    total_size: u64,
    pub peers: Arc<Vec<Peer>>,
    pub metadata: Arc<RwLock<TorrentMetadata>>,
    status: Arc<RwLock<TorrentStatus>>,
    peer_connections: Arc<RwLock<HashMap<Ipv4Addr, Arc<Mutex<TcpStream>>>>>,
    pub pieces_status: Arc<RwLock<BitVec<u8, Lsb0>>>,
    piece_frequency: Arc<RwLock<HashMap<u32, Arc<AtomicU64>>>>,
    piece_hashes: Arc<Vec<[u8; 20]>>,
    is_downloading: AtomicBool,
    path: String,
}

impl Clone for Torrent {
    fn clone(&self) -> Self {
        Torrent {
            info_hash: self.info_hash,
            current_downloaded: AtomicU64::new(self.current_downloaded.load(Ordering::SeqCst)),
            current_uploaded: AtomicU64::new(self.current_uploaded.load(Ordering::SeqCst)),
            total_size: self.total_size,
            peers: Arc::clone(&self.peers),
            metadata: Arc::clone(&self.metadata),
            status: Arc::clone(&self.status),
            peer_connections: Arc::clone(&self.peer_connections),
            pieces_status: Arc::clone(&self.pieces_status),
            piece_frequency: Arc::clone(&self.piece_frequency),
            piece_hashes: Arc::clone(&self.piece_hashes),
            is_downloading: AtomicBool::new(self.is_downloading.load(Ordering::SeqCst)),
            path: self.path.clone(),
        }
    }
}

impl Torrent {
    pub fn new(
        info_hash: [u8; 20],
        total_size: u64,
        peers: Arc<Vec<Peer>>,
        metadata: Arc<RwLock<TorrentMetadata>>,
        pieces_status: Arc<RwLock<BitVec<u8, Lsb0>>>,
        piece_frequency: Arc<RwLock<HashMap<u32, Arc<AtomicU64>>>>,
        piece_hashes: Arc<Vec<[u8; 20]>>,
        is_downloading: AtomicBool,
        path: String,
    ) -> Self {
        Torrent {
            info_hash,
            current_downloaded: AtomicU64::new(0),
            current_uploaded: AtomicU64::new(0),
            total_size,
            peers,
            metadata,
            status: Arc::new(RwLock::new(TorrentStatus::Connecting)),
            peer_connections: Arc::new(RwLock::new(HashMap::new())),
            pieces_status,
            piece_frequency,
            piece_hashes,
            is_downloading,
            path,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        self.status = Arc::new(RwLock::new(TorrentStatus::Connecting));
        let peer_connections = Arc::clone(&self.peer_connections);
        for peer in &*self.peers {
            let peer_clone = peer.clone();
            let peer_connections_clone = Arc::clone(&peer_connections);
            tokio::spawn(async move {
                match TcpStream::connect((peer_clone.ip, peer_clone.port)).await {
                    Ok(stream) => {
                        let mut peer_connections = peer_connections_clone.write().await;
                        peer_connections.insert(peer_clone.ip, Arc::new(Mutex::new(stream)));
                        // Send the handshake message
                        // In the handshake response, you would also receive piece availability info
                        // Update piece_frequency map here
                    }
                    Err(e) => println!("Failed to connect to peer: {:?}", e),
                }
            });
        }

        self.download_and_seed().await?;

        Ok(())
    }

    pub async fn download_and_seed(&mut self) -> Result<()> {
        *self.status.write().await = TorrentStatus::Downloading;

        while let Ok(piece_index) = self.select_rarest_piece().await {
            let mut bad_peers = Vec::new();
            for peer in &*self.peers {
                let piece_data = match self.download_piece_from_peer(peer, piece_index).await {
                    Ok(data) => data,
                    Err(e) => {
                        println!("Failed to download piece from peer: {:?}", e);
                        continue;
                    }
                };

                if !self.validate_piece(&piece_data, piece_index).await {
                    bad_peers.push(peer.clone());
                    continue;
                }

                let metadata = &*self.metadata.read().await;
                // Save the piece to disk
                let path = &self.path;
                match save_piece_to_disk(&piece_data, path, metadata).await {
                    Ok(_) => (),
                    Err(e) => {
                        println!("Error while saving piece to disk: {}", e);
                        continue;
                    }
                }

                // Mark piece as downloaded
                self.pieces_status
                    .write()
                    .await
                    .set(piece_index as usize, true);

                // Update downloaded size
                self.current_downloaded
                    .fetch_add(piece_data.len() as u64, Ordering::SeqCst);

                // Check if the torrent has completed downloading
                if self.current_downloaded.load(Ordering::SeqCst) == self.total_size {
                    println!("Torrent completed!");
                    *self.status.write().await = TorrentStatus::Completed;
                    break;
                }
            }

            for bad_peer in bad_peers {
                self.remove_peer(&bad_peer).await;
            }
        }

        // Start seeding after torrent download is completed
        if *self.status.read().await == TorrentStatus::Completed {
            println!("Start seeding torrent");
            // TODO: Implement seeding logic
        }

        Ok(())
    }

    pub async fn validate_piece(&self, piece: &[u8], piece_index: u32) -> bool {
        let piece_hash = Sha1::digest(piece);
        let expected_hash = self.piece_hashes[piece_index as usize];
        piece_hash.as_slice() == expected_hash
    }

    async fn select_rarest_piece(&self) -> Result<u32> {
        let piece_frequency = self.piece_frequency.read().await;
        let pieces_status = self.pieces_status.read().await;
        let status = Arc::clone(&self.status);

        let rarest_piece_index = (*piece_frequency)
            .iter()
            .filter(|&(k, _v)| !pieces_status[*k as usize])
            .min_by_key(|&(_k, v)| v.load(Ordering::SeqCst))
            .map(|(k, _v)| *k);

        match rarest_piece_index {
            Some(index) => Ok(index),
            None => match pieces_status.all() {
                true => {
                    *status.write().await = TorrentStatus::Completed;
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "All pieces have been retrieved. Torrent is complete.",
                    ))
                }
                false => {
                    *status.write().await = TorrentStatus::Paused;
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "No rare pieces available. Torrent is paused.",
                    ))
                }
            },
        }
    }

    pub async fn pause(&mut self) {
        self.status = Arc::new(RwLock::new(TorrentStatus::Paused));
        let mut peer_connections = self.peer_connections.write().await;
        peer_connections.clear();
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.status = Arc::new(RwLock::new(TorrentStatus::Stopped));
        let mut peer_connections = self.peer_connections.write().await;
        peer_connections.clear();

        Ok(())
    }

    async fn check_status(&self) -> TorrentStatus {
        let status = self.status.read().await;
        *status
    }

    pub async fn download_piece_from_peer(
        &self,
        peer: &Peer,
        piece_index: u32,
    ) -> io::Result<Vec<u8>> {
        // Lock the entire HashMap
        let read_guard = self.peer_connections.read().await;

        // Get the TcpStream from the HashMap
        let connection_arc_mutex = read_guard.get(&peer.ip).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No connection to the specified peer",
            )
        })?;

        // Lock the connection
        let mut connection = connection_arc_mutex.lock().await;

        // Create buffer
        let mut buffer = vec![];

        // Construct and send the request
        let request_message = Message::Request(piece_index);
        connection.write_all(&request_message.encode()).await?;

        // Read the response
        connection.read_to_end(&mut buffer).await?;

        // Process the received message to extract the piece data
        let response_msg = message::identify_message(buffer[0], &buffer[1..]).unwrap();

        // Check if the received message is of the Piece type
        match response_msg {
            Message::Piece(index, begin, data) => {
                if index == piece_index as usize {
                    Ok(data)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        MessageError::MismatchedIndex.to_string(),
                    ))
                }
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                MessageError::InvalidResponse.to_string(),
            )),
        }
    }

    pub async fn remove_peer(&mut self, bad_peer: &Peer) {
        let peers = &mut (*Arc::make_mut(&mut self.peers));
        peers.retain(|peer| *peer != *bad_peer);
    }
}
