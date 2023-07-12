use bincode;
use bitvec::macros::internal::funty::Integral;
use tokio::io::AsyncWriteExt;
use tokio::{
    io,
    net::TcpStream,
    time::{sleep, Duration},
};

use crate::parsing::parser::torrent_metadata::TorrentMetadata;

use super::message::Message;

pub async fn save_piece_to_disk(
    piece_data: &[u8],
    path: &str,
    metadata: &TorrentMetadata,
) -> io::Result<()> {
    // Serialize the metadata
    let encoded_metadata: Vec<u8> = bincode::serialize(metadata).unwrap();

    // Open the metadata file in write mode (overwrite if it exists)
    let mut meta_file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(format!("{}.meta", path))
        .await?;

    // Write the metadata to the metadata file
    meta_file.write_all(&encoded_metadata).await?;

    // Open the data file in append mode (create it if it doesn't exist)
    let data_file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(format!("{}.data", path))
        .await;

    // Write the received piece data to the end of the data file
    data_file?.write_all(piece_data).await?;

    Ok(())
}

pub async fn upload_piece_to_peer(
    peer_address: String,
    piece_index: u32,
    piece_data: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the peer
    let mut stream = TcpStream::connect(peer_address).await?;

    // Craft the `piece` message
    let piece_message = Message::Piece(piece_index as usize, 0, piece_data.to_vec());

    // Send the `piece` message to the peer
    stream.write_all(&piece_message.encode()).await?;

    // Close the connection
    stream.shutdown().await?;

    Ok(())
}

pub async fn upload_piece_to_peer_with_retry(
    peer_address: String,
    piece_index: u32,
    piece_data: &[u8],
    max_retries: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut retries = 0;

    loop {
        match upload_piece_to_peer(peer_address.clone(), piece_index, piece_data).await {
            Ok(()) => {
                println!("Piece uploaded successfully!");
                break;
            }
            Err(err) => {
                retries += 1;

                if retries > max_retries {
                    println!("Max retries exceeded");
                    return Err(err);
                }

                println!(
                    "Failed to upload piece: {}. Retrying in {} seconds...",
                    err,
                    2.pow(retries)
                );
                sleep(Duration::from_secs(2.pow(retries))).await;
            }
        }
    }

    Ok(())
}
