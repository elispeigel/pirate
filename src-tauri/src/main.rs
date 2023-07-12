use app_state::AppState;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

mod app_state;
mod commands;
mod config;
mod hash;
mod message_handling;
mod parsing;
mod torrent_management;
mod tracker;

use torrent_management::{
    peers::{self},
    torrent_manager,
};

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

#[tokio::main]
async fn main() {
    let (async_proc_input_tx, async_proc_input_rx) = mpsc::channel(100);
    let (async_proc_output_tx, _) = mpsc::channel(100);

    let state = AppState {
        torrent_manager: Arc::new(RwLock::new(torrent_manager::TorrentManager::new())),
        async_proc_input_tx: Arc::new(RwLock::new(async_proc_input_tx)),
    };

    tokio::spawn(async_process_model(
        async_proc_input_rx,
        async_proc_output_tx,
    ));

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::add_torrent::add_torrent,
            commands::start_torrent::start_torrent
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
