use crate::torrent_management::torrent::Torrent;
use tokio::sync::RwLockReadGuard;

pub async fn all_pieces_downloaded(torrent: RwLockReadGuard<'_, Torrent>) -> bool {
    // Acquire the read lock before iterating over the pieces_status
    let pieces_status = torrent.pieces_status.read().await;
    pieces_status.iter().all(|status| *status)
}
