pub struct Config {
    pub hash_size: usize,
    pub peer_size: u16,
    pub default_pstr: &'static str,
    pub bittorent_port: String,
    pub tcp_port: u16,
    pub array_size: usize,
}
impl Config {
    pub fn new() -> Config {
        Config {
            hash_size: 20,
            peer_size: 6,
            default_pstr: "BitTorrent protocol",
            bittorent_port: "6881".to_string(),
            tcp_port: 80,
            array_size: 20,
        }
    }
}
