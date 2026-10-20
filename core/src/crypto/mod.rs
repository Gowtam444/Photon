use snow::{Builder, HandshakeState, TransportState};
use snow::params::NoiseParams;
use thiserror::Error;
pub const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
pub const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB
pub const MAX_MESSAGE_LEN: usize = CHUNK_SIZE + 1024; // Chunk + Poly1305 Tag + Framing Overhead

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Noise error: {0}")]
    NoiseError(#[from] snow::Error),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid handshake state")]
    InvalidState,
    #[error("Key mismatch")]
    KeyMismatch,
    #[error("Message too large")]
    MessageTooLarge,
}

pub struct KeyPair {
    pub private: Vec<u8>,
    pub public: Vec<u8>,
}

impl KeyPair {
    pub fn generate() -> Result<Self, CryptoError> {
        let builder = Builder::new(NOISE_PATTERN.parse().unwrap());
        let keypair = builder.generate_keypair()?;
        Ok(Self {
            private: keypair.private,
            public: keypair.public,
        })
    }
}

pub struct PhotonCrypto {
    params: NoiseParams,
    keypair: KeyPair,
}

impl PhotonCrypto {
    pub fn new(keypair: KeyPair) -> Self {
        let params: NoiseParams = NOISE_PATTERN.parse().unwrap();
        Self { params, keypair }
    }

    pub fn builder(&self) -> Result<Builder<'_>, CryptoError> {
        Ok(Builder::new(self.params.clone())
            .local_private_key(&self.keypair.private)?)
    }

    pub fn start_initiator(&self) -> Result<HandshakeState, CryptoError> {
        Ok(self.builder()?.build_initiator()?)
    }

    pub fn start_responder(&self) -> Result<HandshakeState, CryptoError> {
        Ok(self.builder()?.build_responder()?)
    }

    pub fn get_public_key_fingerprint(&self) -> String {
        hex::encode(blake3::hash(&self.keypair.public).as_bytes())
    }
}

pub struct Session {
    transport: TransportState,
    remote_static_key: Vec<u8>,
    handshake_hash: Vec<u8>,
}

impl Session {
    pub fn new(transport: TransportState, remote_static_key: Vec<u8>, handshake_hash: Vec<u8>) -> Self {
        Self {
            transport,
            remote_static_key,
            handshake_hash,
        }
    }

    pub fn encrypt(&mut self, plaintext: &[u8], ciphertext: &mut [u8]) -> Result<usize, CryptoError> {
        if plaintext.len() > MAX_MESSAGE_LEN {
            return Err(CryptoError::MessageTooLarge);
        }
        Ok(self.transport.write_message(plaintext, ciphertext)?)
    }

    pub fn decrypt(&mut self, ciphertext: &[u8], plaintext: &mut [u8]) -> Result<usize, CryptoError> {
         if ciphertext.len() > MAX_MESSAGE_LEN {
            return Err(CryptoError::MessageTooLarge);
        }
        Ok(self.transport.read_message(ciphertext, plaintext)?)
    }

    pub fn remote_pubkey(&self) -> &[u8] {
        &self.remote_static_key
    }

    pub fn get_sas_code(&self) -> String {
        // Derive 6-digit code from handshake hash
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"sas");
        hasher.update(&self.handshake_hash);
        let hash = hasher.finalize();

        let bytes = hash.as_bytes();
        let num = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        format!("{:06}", num % 1000000)
    }
}
