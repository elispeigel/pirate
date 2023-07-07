use crate::parsing::parser::TorrentMetadata;
use crate::peers::Peer;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Torrent {
    info_hash: [u8; 20],
    current_downloaded: u64,
    current_uploaded: u64,
    total_size: u64,
    pub peers: Arc<Vec<Peer>>,
    pub metadata: Arc<Mutex<TorrentMetadata>>,
    status: TorrentStatus,
    peer_connections: Vec<Arc<Mutex<TcpStream>>>,
    pub pieces_status: Arc<Mutex<Vec<bool>>>,
}

#[derive(Clone, Copy)]
pub enum TorrentStatus {
    Initialized,
    Connecting,
    Downloading,
    Seeding,
    Paused,
    Stopped,
    Completed,
}

impl Torrent {
    pub fn new(
        info_hash: [u8; 20],
        total_size: u64,
        peers: Arc<Vec<Peer>>,
        metadata: Arc<Mutex<TorrentMetadata>>,
        pieces_status: Arc<Mutex<Vec<bool>>>,
    ) -> Self {
        Torrent {
            info_hash,
            current_downloaded: 0,
            current_uploaded: 0,
            total_size,
            peers,
            metadata,
            status: TorrentStatus::Initialized,
            peer_connections: Vec::new(),
            pieces_status,
        }
    }

    async fn start(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {        self.status = TorrentStatus::Connecting;
        for peer in &*self.peers {
            // Assume the peer is a tuple of (ip: IpAddr, port: u16)
            match TcpStream::connect((peer.ip, peer.port)).await {
                Ok(stream) => {
                    self.peer_connections.push(Arc::new(Mutex::new(stream)));
                    // send the handshake message...
                }
                Err(e) => println!("Failed to connect to peer: {:?}", e),
            }
        }
        Ok(())
    }

    fn pause(&mut self) {
        self.status = TorrentStatus::Paused;
        self.peer_connections.clear();
    }

    fn stop(&mut self) {
        self.status = TorrentStatus::Stopped;
        self.peer_connections.clear();
    }

    fn check_status(&self) -> TorrentStatus {
        self.status
    }
}

pub struct TorrentManager {
    torrents: HashMap<String, Arc<Mutex<Torrent>>>,
}

impl TorrentManager {
    pub fn new() -> Self {
        Self {
            torrents: HashMap::new(),
        }
    }
    pub fn add_torrent(&mut self, torrent_hash: String, torrent: Arc<Mutex<Torrent>>) {
        self.torrents.insert(torrent_hash, torrent);
    }    

    pub async fn start_torrent(&self, torrent_hash: &str) -> Result<(), String> {
        if let Some(torrent) = self.torrents.get(torrent_hash) {
            let mut torrent = torrent.lock().await;
            match torrent.start().await {
                Ok(_) => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err("Torrent not found".to_string())
        }
    }
}
