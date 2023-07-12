use std::sync::Arc;
use tokio::sync::RwLock;

use crate::torrent_management;

pub struct AppState {
    pub torrent_manager: Arc<RwLock<torrent_management::torrent_manager::TorrentManager>>,
    pub async_proc_input_tx: Arc<RwLock<tauri::async_runtime::Sender<String>>>,
}
