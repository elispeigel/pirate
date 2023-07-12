use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::torrent::Torrent;

pub struct TorrentManager {
    torrents: HashMap<String, Arc<RwLock<Torrent>>>,
}

impl TorrentManager {
    pub fn new() -> Self {
        Self {
            torrents: HashMap::new(),
        }
    }
    pub fn add_torrent(&mut self, torrent_hash: String, torrent: Arc<RwLock<Torrent>>) {
        self.torrents.insert(torrent_hash, torrent);
    }

    pub async fn start_torrent(&self, torrent_hash: &str) -> Result<(), String> {
        if let Some(torrent) = self.torrents.get(torrent_hash) {
            let torrent = torrent.clone();
            let mut torrent_guard = torrent.write().await;
            match torrent_guard.start().await {
                Ok(_) => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err("Torrent not found".to_string())
        }
    }

    pub async fn pause_torrent(&self, torrent_hash: &str) -> Result<(), String> {
        if let Some(torrent) = self.torrents.get(torrent_hash) {
            let torrent = torrent.clone();
            let mut torrent_guard = torrent.write().await;
            torrent_guard.pause().await;
            Ok(())
        } else {
            Err("Torrent not found".to_string())
        }
    }

    pub async fn stop_torrent(&self, torrent_hash: &str) -> Result<(), String> {
        if let Some(torrent) = self.torrents.get(torrent_hash) {
            let torrent = torrent.clone();
            let mut torrent_guard = torrent.write().await;
            torrent_guard.stop().await.map_err(|e| e.to_string())
        } else {
            Err("Torrent not found".to_string())
        }
    }
}
