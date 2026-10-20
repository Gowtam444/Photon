# SECURITY.md - Threat Model and Security Guidelines

## Threat Model Overview

Photon is an end-to-end encrypted file transfer app designed to securely transfer files over local networks (Wi-Fi, hotspot). The primary goal is to ensure confidentiality, integrity, and authenticity of files transferred between devices.

### In Scope

- **Confidentiality:** E2EE using Noise Protocol (`Noise_XX_25519_ChaChaPoly_BLAKE2s`). Traffic cannot be passively sniffed on an open Wi-Fi network.
- **Integrity:** Every 1 MiB chunk is AEAD encrypted. Furthermore, an overall BLAKE3 hash is verified to guarantee file integrity upon assembly.
- **Authenticity:** Short Authentication Strings (SAS) are used on first connection to thwart active Man-in-the-Middle (MitM) attacks. Post-verification, public keys are pinned (Trust on First Use - TOFU).
- **Forward Secrecy:** Provided by the Noise XX handshake.
- **Path Traversal Protection:** All received filenames are sanitized. Files are only ever saved to the designated incoming folder.
- **Memory Safety:** Rust handles memory bounds. Frame size limits prevent OOM DoS attacks.
- **Replay/Reordering:** Handled by Noise protocol state and sequenced chunk numbers.

### Out of Scope

- **Anonymity:** Devices broadcast their presence via mDNS (including a public key fingerprint).
- **Device Compromise:** If a device is infected with malware, it could extract the static private keys stored in the OS keyring or read decrypted files.
- **Traffic Analysis:** An attacker can observe the volume and timing of data to infer file sizes.

## Cryptography Specifications

- **Handshake:** Noise Protocol `Noise_XX_25519_ChaChaPoly_BLAKE2s` (via the audited `snow` crate).
- **Long-term Keys:** X25519 static keypairs generated on first run.
- **Chunk Encryption:** AEAD (ChaCha20Poly1305 via `snow`'s transport mode) applied per chunk.
- **Hashing:** BLAKE3 for individual chunks and whole file hashes (fast and secure).
- **Key Storage:** Native OS keystores (Android Keystore / Windows DPAPI).

## Peer Authentication

1. Upon first transfer attempt, devices exchange ephemeral and static public keys during the Noise XX handshake.
2. Both devices display a 6-digit code derived from the handshake hash (or via QR code verification).
3. The users must confirm the code matches.
4. After confirmation, the static key is pinned. Future connections to the same device bypass the manual check.
5. If a pinned key changes, the connection is immediately aborted, and a loud warning is shown to the user.
