use std::fs::OpenOptions;
use std::io::{self, Seek, SeekFrom, Write};

use super::message_error::MessageError;
use crate::torrent_management::{message::Message, peers::Peer, torrent::Torrent};

// Handle the message received accordingly and manipulate the TorrentMetadata.
pub async fn message_handler(
    msg: Message,
    peer: &Peer,
    torrent: &mut Torrent,
) -> Result<(), MessageError> {
    let piece_index = match msg {
        Message::Piece(index, _, _) => index,
        _ => 0,
    };

    match msg {
        Message::Choke => {
            println!("Choke message received.");
            Ok(())
        }
        Message::Unchoke => {
            println!("Unchoke message received.");
            Ok(())
        }
        Message::Interested => {
            println!("Interested message received.");
            Ok(())
        }
        Message::NotInterested => {
            println!("NotInterested message received.");
            Ok(())
        }
        Message::Have => {
            println!("Have message received.");
            Ok(())
        }
        Message::Bitfield(body) => {
            bitfield_handler(body)?;
            Ok(())
        }
        Message::Request(piece_index) => {
            let metadata = &*torrent.metadata.read().await;
            let piece_size = metadata.info.piece_length as usize;
            let file_path = metadata
                .file_path
                .to_str()
                .ok_or(MessageError::NonUnicodePath)?;

            // Acquire the read lock before checking the piece status
            let pieces_status = torrent.pieces_status.read().await;

            // Check if piece is already downloaded
            if pieces_status[piece_index as usize] {
                println!(
                    "Piece {} already downloaded. Ignoring request.",
                    piece_index
                );
                return Ok(());
            }

            // Assuming a peer object is available and it has a method to read a piece
            let piece_data: Vec<u8> = torrent.download_piece_from_peer(&peer, piece_index).await?;

            // Now, save the downloaded piece data to the file
            let write_result = tokio::task::block_in_place(|| {
                write_to_file(
                    piece_index.try_into().unwrap(),
                    0,
                    piece_size,
                    piece_data,
                    file_path,
                )
                .map_err(|_| MessageError::FileIOError)
            });

            match write_result {
                Ok(_) => {
                    // Acquire the write lock before setting the piece status
                    let mut pieces_status = torrent.pieces_status.write().await;
                    pieces_status.set(piece_index as usize, true);
                    if pieces_status.all() {
                        println!("Download complete!");
                    }
                    Ok(())
                }
                Err(_) => Err(MessageError::FileIOError),
            }
        }
        Message::Piece(index, begin, data) => {
            let metadata = &*torrent.metadata.read().await;
            let piece_size = metadata.info.piece_length as usize;
            let file_path = metadata
                .file_path
                .to_str()
                .ok_or(MessageError::NonUnicodePath)?;
            let write_result = tokio::task::block_in_place(|| {
                write_to_file(index, begin, piece_size, data, file_path)
                    .map_err(|_| MessageError::FileIOError)
            });

            match write_result {
                Ok(_) => {
                    // Acquire the write lock before setting the piece status
                    let mut pieces_status = torrent.pieces_status.write().await;
                    pieces_status.set(piece_index as usize, true);
                    if pieces_status.all() {
                        println!("Download complete!");
                    }
                    Ok(())
                }
                Err(_) => Err(MessageError::FileIOError),
            }
        }
        Message::Cancel => {
            println!("Cancel message received.");
            Ok(())
        }
        Message::KeepAlive => {
            println!("Received KeepAlive");
            Ok(())
        }
    }
}

// Writes the received data to a file
fn write_to_file(
    index: usize,
    begin: usize,
    piece_size: usize,
    data: Vec<u8>,
    file_path: &str,
) -> io::Result<()> {
    let position = index * piece_size + begin;

    let mut file = OpenOptions::new().write(true).open(file_path)?;

    file.seek(SeekFrom::Start(position as u64))?;

    file.write_all(&data)?;

    Ok(())
}

fn bitfield_handler(bitfield: Vec<u8>) -> Result<(), MessageError> {
    println!("Bitfield: {:?}", bitfield);
    Ok(())
}

