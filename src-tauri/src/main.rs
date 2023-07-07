use serde_bencode::to_bytes;
use serde_bencode::{de, value::Value};
use sha1::{Digest, Sha1};
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

mod hash;
mod parsing;
mod torrent_management;
mod tracker;

use torrent_management::{peers, torrent};

struct AppState {
    torrent_manager: Arc<Mutex<torrent_management::torrent::TorrentManager>>,
    async_proc_input_tx: tokio::sync::Mutex<tauri::async_runtime::Sender<String>>,
}

async fn async_process_model(
    mut input_rx: mpsc::Receiver<String>,
    output_tx: mpsc::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        while let Some(input) = input_rx.recv().await {
            let output = input;
            output_tx.send(output).await.unwrap();
        }
    }
}

pub fn all_pieces_downloaded(pieces_status: &Vec<bool>) -> bool {
    pieces_status.iter().all(|&status| status)
}

pub fn parse_pieces_status(pieces_status: Value) -> Result<Vec<bool>, ()> {
    // Extract the ByteString from Bencode
    let pieces_status_bytes = match pieces_status {
        Value::Bytes(s) => s,
        _ => return Err(()),
    };

    // Convert ByteString into Vec<bool>
    let pieces_status_bits: Vec<bool> = pieces_status_bytes
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |bit| byte & (1 << bit) != 0))
        .collect();

    Ok(pieces_status_bits)
}

#[tauri::command]
async fn add_torrent(
    state: tauri::State<'_, AppState>,
    torrent_file: String,
) -> Result<String, String> {
    // Read the torrent file
    let torrent_data = std::fs::read(&torrent_file).map_err(|e| e.to_string())?;

    let torrent_data = match de::from_bytes::<Value>(&torrent_data) {
        Ok(Value::Dict(dict)) => dict,
        Ok(_) => return Err("Top level bencode should be a dict".into()),
        Err(e) => return Err(format!("Failed to parse torrent file: {}", e)),
    };

    let pieces_status = match torrent_data.remove(&b"pieces".to_vec()) {
        Some(raw_status) => match parse_pieces_status(raw_status) {
            Ok(status) => Arc::new(Mutex::new(status)),
            Err(_) => return Err("Failed to parse pieces status".to_string()),
        },
        None => return Err("Failed to find 'pieces' field".to_string()),
    };

    let info = match torrent_data.remove(&b"info".to_vec()) {
        Some(info) => info,
        None => return Err("Failed to find 'info' field".to_string()),
    };

    let info_bytes = match to_bytes(&info) {
        Ok(bytes) => bytes,
        Err(e) => return Err(format!("Failed to serialize 'info' field: {}", e)),
    };

    let mut hasher = Sha1::new();
    hasher.update(info_bytes);
    let info_hash = hasher.finalize();
    let info_hash: [u8; 20] = info_hash.as_slice().try_into().expect("Wrong length");

    let metadata = parsing::parser::parse_bencoded_torrent(torrent_data);

    let total_size = metadata.total_size.unwrap_or_default(); 
    
    let peers = match peers::get_peers(&metadata) {
        Ok(peers) => peers,
        Err(e) => return Err(format!("Failed to get peers: {}", e))
    };


    // The rest of the code including info_hash, total_size, peers and torrent computations
    let torrent = torrent::Torrent::new(
        info_hash,
        total_size,
        peers,
        metadata.clone(),
        pieces_status.clone(),
    );

    // Save the pieces status into the Torrent struct
    torrent.pieces_status = pieces_status.clone();

    // Add the torrent
    state
        .torrent_manager
        .lock()
        .await
        .add_torrent(torrent_data, torrent.clone());

    // Now whenever you need to check if all pieces are downloaded, use all_pieces_downloaded function:
    if all_pieces_downloaded(&pieces_status.lock().await) {
        println!("All pieces are downloaded!");
    }
}

#[tauri::command]
async fn start_torrent(
    state: tauri::State<'_, AppState>,
    torrent_hash: String,
) -> Result<String, String> {
    match state
        .torrent_manager
        .lock()
        .await
        .start_torrent(&torrent_hash)
        .await
    {
        Ok(_) => Ok("Successfully started torrent!".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[tokio::main]
async fn main() {
    let (async_proc_input_tx, async_proc_input_rx) = mpsc::channel(100);
    let (async_proc_output_tx, _) = mpsc::channel(100);

    let state = AppState {
        torrent_manager: Arc::new(Mutex::new(torrent::TorrentManager::new())),
        async_proc_input_tx: tokio::sync::Mutex::new(async_proc_input_tx),
    };

    tokio::spawn(async_process_model(
        async_proc_input_rx,
        async_proc_output_tx,
    ));

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![add_torrent, start_torrent])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
