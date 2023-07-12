use core::sync::atomic::AtomicBool;
use serde_bencode::{de, value::Value};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    app_state::AppState,
    config,
    hash::compute_info_hash,
    parsing::parser::parse_error::parse_bencoded_torrent,
    torrent_management::{peers::get_peers, torrent},
};

use super::{
    all_pieces_downloaded::all_pieces_downloaded, parse_pieces_status::parse_pieces_status,
};

#[tauri::command]
pub async fn add_torrent(
    state: tauri::State<'_, AppState>,
    torrent_file: String,
) -> Result<String, String> {
    let configuration = config::Config::new();
    let hash_size = configuration.hash_size;
    // Prepare a separate lock to ensure atomic operations when updating `torrent_manager` and `pieces_status`.
    let torrent_operation_lock = RwLock::new(());

    let torrent_data = std::fs::read(&torrent_file).map_err(|e| e.to_string())?;

    let metadata_result = parse_bencoded_torrent(torrent_data.clone());

    let torrent_data = match de::from_bytes::<Value>(&torrent_data) {
        Ok(Value::Dict(dict)) => dict,
        Ok(_) => return Err("Top level bencode should be a dict".into()),
        Err(e) => return Err(format!("Failed to parse torrent file: {}", e)),
    };

    // Apply the lock here for atomic operations
    let _guard = torrent_operation_lock.write().await;

    let pieces_status = match torrent_data.get(&b"pieces".to_vec()) {
        Some(raw_status) => {
            let raw_status_bytes = convert_to_bytes(raw_status.clone());
            match parse_pieces_status(&raw_status_bytes) {
                Ok(status) => Arc::new(RwLock::new(status)),
                Err(_) => return Err("Failed to parse pieces status".to_string()),
            }
        }
        None => return Err("Failed to find 'pieces' field".to_string()),
    };

    let info = match torrent_data.get(&b"info".to_vec()) {
        Some(info) => info,
        None => return Err("Failed to find 'info' field".to_string()),
    };

    let info_hash = match compute_info_hash(info) {
        Ok(hash) => hash,
        Err(e) => return Err(format!("Failed to compute info hash: {}", e)),
    };

    if info_hash.len() != hash_size {
        return Err("Incorrect hash length".to_string());
    }

    let info_hash_array: [u8; 20] = {
        let mut array = [0; 20];
        for (index, &value) in info_hash.iter().enumerate() {
            array[index] = value;
        }
        array
    };

    let total_size: u64 = match torrent_data.get(&b"info".to_vec()) {
        Some(Value::Dict(info_dict)) => {
            match info_dict.get(&b"length".to_vec()) {
                Some(Value::Int(length)) => *length as u64, // single-file torrent
                None => {
                    // Maybe it's a multi-file torrent
                    match info_dict.get(&b"files".to_vec()) {
                        Some(Value::List(files)) => files
                            .iter()
                            .map(|file| match file {
                                Value::Dict(dict) => match dict.get(&b"length".to_vec()) {
                                    Some(Value::Int(length)) => *length as u64,
                                    _ => 0,
                                },
                                _ => 0,
                            })
                            .sum(),
                        _ => {
                            return Err(
                                "Invalid or missing 'files' field in torrent data".to_string()
                            )
                        }
                    }
                }
                _ => return Err("Invalid or missing 'length' field in torrent data".to_string()),
            }
        }
        _ => return Err("Invalid or missing 'info' field in torrent data".to_string()),
    };

    let (metadata, peers) = match metadata_result {
        Ok(data) => {
            let peers = get_peers(&data).await?;
            (Arc::new(RwLock::new(data)), peers)
        }
        Err(e) => return Err(format!("Failed to parse metadata: {}", e)),
    };

    let piece_frequency = Arc::new(RwLock::new(HashMap::new()));
    let piece_hashes = Arc::new(Vec::new());
    let is_downloading = AtomicBool::new(false);
    let path = torrent_file.clone();

    let torrent = torrent::Torrent::new(
        info_hash_array,
        total_size,
        Arc::new(peers),
        metadata,
        pieces_status,
        piece_frequency.clone(),
        piece_hashes.clone(),
        is_downloading,
        path,
    );

    // Then, apply the lock before updating `torrent_manager`.
    let mut torrent_manager = state.torrent_manager.write().await;
    let torrent_hash = info_hash_array
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>();
    let torrent = Arc::new(RwLock::new(torrent));
    torrent_manager.add_torrent(torrent_hash, torrent.clone());

    // Release the lock right after the operation completed
    drop(torrent_manager);

    let torrent_guard = torrent.read().await;

    if all_pieces_downloaded(torrent_guard).await {
        println!("All pieces are downloaded!");
    }

    Ok("Torrent added successfully".to_string())
}

fn convert_to_bytes(value: Value) -> Vec<u8> {
    match serde_bencode::to_bytes(&value) {
        Ok(v) => v,
        Err(e) => panic!("Failed to serialize Value to bytes: {}", e),
    }
}
