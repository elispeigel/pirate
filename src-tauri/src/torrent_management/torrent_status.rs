#[derive(Clone, Copy, PartialEq)]
pub enum TorrentStatus {
    Initialized,
    Connecting,
    Downloading,
    Seeding,
    Paused,
    Stopped,
    Completed,
}
