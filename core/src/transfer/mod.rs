use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write, Read};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use thiserror::Error;
use bytes::{BytesMut, BufMut};
use log::{info, error, debug};
use blake3::Hasher;

use crate::crypto::{CryptoError, Session, CHUNK_SIZE};
use self::frame::{Message, FileMeta};

pub mod frame;

#[derive(Error, Debug)]
pub enum TransferError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Integrity error: hash mismatch for file {0}")]
    Integrity(String),
    #[error("Security error: {0}")]
    Security(String),
    #[error("Transfer cancelled")]
    Cancelled,
}

pub struct TransferManager {
    base_dir: PathBuf,
}

impl TransferManager {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Read an encrypted message and decrypt it
    async fn read_msg(stream: &mut TcpStream, session: &mut Session) -> Result<Message, TransferError> {
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > crate::crypto::MAX_MESSAGE_LEN {
            return Err(TransferError::Security("Message too large".into()));
        }

        let mut ciphertext = vec![0u8; len];
        stream.read_exact(&mut ciphertext).await?;

        let mut plaintext = vec![0u8; len];
        let p_len = session.decrypt(&ciphertext, &mut plaintext)?;
        plaintext.truncate(p_len);

        let msg: Message = bincode::deserialize(&plaintext)?;
        Ok(msg)
    }

    /// Encrypt and send a message
    async fn send_msg(stream: &mut TcpStream, session: &mut Session, msg: &Message) -> Result<(), TransferError> {
        let plaintext = bincode::serialize(msg)?;

        // Output buffer for ciphertext must be larger by the Poly1305 tag size (16 bytes)
        let mut ciphertext = vec![0u8; plaintext.len() + 16];
        let c_len = session.encrypt(&plaintext, &mut ciphertext)?;
        ciphertext.truncate(c_len);

        stream.write_all(&(c_len as u32).to_be_bytes()).await?;
        stream.write_all(&ciphertext).await?;
        Ok(())
    }

    fn sanitize_path(&self, requested_path: &str) -> Result<PathBuf, TransferError> {
        let mut path = PathBuf::new();
        // Disallow absolute paths, parent directory traversals, and null bytes
        for component in Path::new(requested_path).components() {
            match component {
                std::path::Component::Normal(c) => {
                    let s = c.to_str().ok_or_else(|| TransferError::Security("Invalid path encoding".into()))?;
                    if s.contains('\0') {
                        return Err(TransferError::Security("Null byte in path".into()));
                    }
                    path.push(c);
                }
                _ => return Err(TransferError::Security("Path traversal detected".into())),
            }
        }

        let final_path = self.base_dir.join(path);
        Ok(final_path)
    }
}
