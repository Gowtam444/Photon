use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    /// Initial Noise handshake message
    HandshakeInit(Vec<u8>),

    /// Response Noise handshake message
    HandshakeResponse(Vec<u8>),

    /// Final confirmation that SAS matched or pairing is accepted
    ConfirmPairing,

    /// Request to send files
    TransferRequest {
        files: Vec<FileMeta>,
        total_size: u64,
    },

    /// Accepted or rejected by user
    TransferResponse {
        accepted: bool,
        message: Option<String>,
    },

    /// Resume request sending the number of valid chunks already received
    ResumeRequest {
        file_id: u64,
        chunk_offset: u64,
    },

    /// Send a file chunk (encrypted payload is outside this enum in actual stream)
    /// This header goes first, followed by the encrypted chunk bytes.
    ChunkHeader {
        file_id: u64,
        chunk_index: u64,
        length: u32,
    },

    /// Indicates completion of a file
    FileComplete {
        file_id: u64,
        blake3_hash: String,
    },

    /// Error during transfer
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileMeta {
    pub id: u64,
    pub path: String, // sanitized relative path
    pub size: u64,
    pub blake3_hash: Option<String>, // if known ahead of time, else None
}
