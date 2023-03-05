// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


mod read_torrents;

fn main() {
    tauri::Builder::default()
    // This is where you pass in your commands
        .invoke_handler(tauri::generate_handler![read_torrents::read_torrent_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
