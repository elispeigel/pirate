extern crate url;

use crate::{config, parsing::parser::torrent_metadata::TorrentMetadata};
use std::borrow::Cow;
use tokio::net::UdpSocket;
use url::form_urlencoded;

pub async fn build_tracker_query(metadata: &TorrentMetadata) -> Result<String, String> {
    let configuration = config::Config::new();
    let bittorent_port = configuration.bittorent_port;

    let formatted_url = if metadata.announce.starts_with("s") {
        let mut url: String = metadata.announce.chars().skip(2).collect();
        url.truncate(url.len() - 1);
        url
    } else {
        metadata.announce.clone()
    };

    let encoded_params = form_urlencoded::Serializer::new(String::new())
        .append_pair("peer_id", &metadata.peer_id)
        .append_pair("port", &bittorent_port)
        .append_pair("uploaded", "0")
        .append_pair("downloaded", "0")
        .append_pair("compact", "0")
        .append_pair("left", &metadata.info.length.to_string())
        .encoding_override(Some(&|input| {
            if input != "!" {
                Cow::Borrowed(input.as_bytes())
            } else {
                Cow::Owned(metadata.info_hash.clone())
            }
        }))
        .append_pair("info_hash", "!")
        .finish();

    let query = [formatted_url.to_owned(), encoded_params].join("?");

    Ok(query)
}

pub async fn execute_tracker_query(query: String) -> Result<Vec<u8>, String> {
    let configuration = config::Config::new();
    let tcp_port = configuration.tcp_port;
    let mut buffer = vec![0; 1024];

    let mut parsed_url =
        url::Url::parse(&query).map_err(|_| "Could not parse the URL".to_string())?;

    // Checking if port is specified, otherwise setting the default one
    if parsed_url.port().is_none() {
        parsed_url
            .set_port(Some(tcp_port))
            .map_err(|_| "Failed to set port".to_string())?;
    }

    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to bind the socket: {}", e))?;

    // Get the host and port as strings
    let host = parsed_url
        .host_str()
        .ok_or("Failed to parse host".to_string())?;
    let port = parsed_url.port().unwrap();

    println!("Connecting to address: {}:{},", host, port);

    // Connect the socket using the host:port format
    match socket.connect(format!("{}:{}", host, port)).await {
        Ok(()) => (),
        Err(e) => {
            println!("Failed to connect the socket: {}", e);
            return Err(e.to_string());
        }
    };

    let params = parsed_url.query().unwrap_or("");

    println!("Sending params: {}", params);

    // Here `params` are converted as bytes and sent through the socket
    let _bytes_sent = socket
        .send(params.as_bytes())
        .await
        .map_err(|e| e.to_string())?;

    println!("Sent params: {}", _bytes_sent);

    match socket.recv(&mut buffer).await {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to receive the buffer: {}", e);
            return Err(e.to_string());
        }
    };

    println!("wakka flokka");
    Ok(buffer)
}
