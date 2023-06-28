// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hash;
mod decoder;
mod parser;
mod tracker;
mod peer_connection;
mod message;

use std::fs;

#[tauri::command]
fn start() {
    const TORRENT_PATH: &str = "/Users/elispeigel/code/pirate/puppy.torrent";
    
    let bencoded_metadata: Vec<u8> = fs::read(TORRENT_PATH).unwrap();
    let metadata = parser::parse_bencoded_torrent(bencoded_metadata).unwrap();
    let peers = match tracker::get_peers(&metadata) {
        Ok(peers) => peers,
        Err(e) => panic!("{}", e),
    };
    
    peer_connection::connect_to_peers(&peers, &metadata);
}

fn main() {
    tauri::Builder::default()
    // This is where you pass in your commands
        .invoke_handler(tauri::generate_handler![start])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
