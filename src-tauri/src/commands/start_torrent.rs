use crate::app_state::AppState;

#[tauri::command]
pub async fn start_torrent(
    state: tauri::State<'_, AppState>,
    torrent_hash: String,
) -> Result<String, String> {
    let torrent_manager = state.torrent_manager.read().await;
    match torrent_manager.start_torrent(&torrent_hash).await {
        Ok(_) => Ok("Successfully started torrent!".to_string()),
        Err(e) => Err(e.to_string()),
    }
}
