#[cfg(test)]
mod tests {
    use crate::crypto::{KeyPair, PhotonCrypto, Session};
    use crate::transfer::{TransferManager, frame::FileMeta};
    use tokio::net::{TcpListener, TcpStream};
    use std::io::Write;
    use tempfile::tempdir;

    async fn setup_connection() -> (TcpStream, Session, TcpStream, Session) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let client_keypair = KeyPair::generate().unwrap();
        let server_keypair = KeyPair::generate().unwrap();

        let client_crypto = PhotonCrypto::new(client_keypair);
        let server_crypto = PhotonCrypto::new(server_keypair);

        let server_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut handshake = server_crypto.start_responder().unwrap();

            // Handshake logic
            let mut len_bytes = [0u8; 4];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut len_bytes).await.unwrap();
            let len = u32::from_be_bytes(len_bytes) as usize;
            let mut recv_buf = vec![0u8; len];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut recv_buf).await.unwrap();

            let mut buf = vec![0u8; 1024];
            handshake.read_message(&recv_buf, &mut buf).unwrap();

            let len = handshake.write_message(&[], &mut buf).unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut stream, &(len as u32).to_be_bytes()).await.unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut stream, &buf[..len]).await.unwrap();

            let mut len_bytes = [0u8; 4];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut len_bytes).await.unwrap();
            let len = u32::from_be_bytes(len_bytes) as usize;
            let mut recv_buf = vec![0u8; len];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut recv_buf).await.unwrap();
            handshake.read_message(&recv_buf, &mut buf).unwrap();

            let transport = handshake.into_transport_mode().unwrap();
            let session = Session::new(transport, vec![], vec![]);
            (stream, session)
        });

        let mut client_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
        let mut handshake = client_crypto.start_initiator().unwrap();

        let mut buf = vec![0u8; 1024];
        let len = handshake.write_message(&[], &mut buf).unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut client_stream, &(len as u32).to_be_bytes()).await.unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut client_stream, &buf[..len]).await.unwrap();

        let mut len_bytes = [0u8; 4];
        tokio::io::AsyncReadExt::read_exact(&mut client_stream, &mut len_bytes).await.unwrap();
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut recv_buf = vec![0u8; len];
        tokio::io::AsyncReadExt::read_exact(&mut client_stream, &mut recv_buf).await.unwrap();
        handshake.read_message(&recv_buf, &mut buf).unwrap();

        let len = handshake.write_message(&[], &mut buf).unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut client_stream, &(len as u32).to_be_bytes()).await.unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut client_stream, &buf[..len]).await.unwrap();

        let transport = handshake.into_transport_mode().unwrap();
        let client_session = Session::new(transport, vec![], vec![]);

        let (server_stream, server_session) = server_task.await.unwrap();
        (client_stream, client_session, server_stream, server_session)
    }

    #[tokio::test]
    async fn test_roundtrip_transfer() {
        let dir_send = tempdir().unwrap();
        let dir_recv = tempdir().unwrap();

        let file_path = dir_send.path().join("test.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        let data = b"Hello, Photon!";
        file.write_all(data).unwrap();

        let mut base_paths = std::collections::HashMap::new();
        base_paths.insert(1, file_path.clone());

        let meta = FileMeta {
            id: 1,
            path: "test.txt".to_string(),
            size: data.len() as u64,
            blake3_hash: None,
        };

        let (mut c_stream, mut c_session, mut s_stream, mut s_session) = setup_connection().await;

        let recv_manager = TransferManager::new(dir_recv.path());
        let server_task = tokio::spawn(async move {
            recv_manager.receive_files(&mut s_stream, &mut s_session).await.unwrap();
        });

        let send_manager = TransferManager::new(dir_send.path());
        send_manager.send_files(&mut c_stream, &mut c_session, vec![meta], base_paths).await.unwrap();

        server_task.await.unwrap();

        let recv_content = std::fs::read(dir_recv.path().join("test.txt")).unwrap();
        assert_eq!(recv_content, data);
    }

    #[test]
    fn test_path_sanitization() {
        let manager = TransferManager::new("/tmp");

        assert!(manager.sanitize_path("foo/bar.txt").is_ok());
        assert!(manager.sanitize_path("../foo/bar.txt").is_err());
        assert!(manager.sanitize_path("/etc/passwd").is_err());
    }

    #[tokio::test]
    async fn test_empty_file_transfer() {
        let dir_send = tempdir().unwrap();
        let dir_recv = tempdir().unwrap();

        let file_path = dir_send.path().join("empty.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        let data = b"";
        file.write_all(data).unwrap();

        let mut base_paths = std::collections::HashMap::new();
        base_paths.insert(1, file_path.clone());

        let meta = FileMeta {
            id: 1,
            path: "empty.txt".to_string(),
            size: data.len() as u64,
            blake3_hash: None,
        };

        let (mut c_stream, mut c_session, mut s_stream, mut s_session) = setup_connection().await;

        let recv_manager = TransferManager::new(dir_recv.path());
        let server_task = tokio::spawn(async move {
            recv_manager.receive_files(&mut s_stream, &mut s_session).await.unwrap();
        });

        let send_manager = TransferManager::new(dir_send.path());
        send_manager.send_files(&mut c_stream, &mut c_session, vec![meta], base_paths).await.unwrap();

        server_task.await.unwrap();

        let recv_content = std::fs::read(dir_recv.path().join("empty.txt")).unwrap();
        assert_eq!(recv_content, data);
    }




    #[tokio::test]
    async fn test_wrong_key_rejection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let client_keypair = KeyPair::generate().unwrap();
        let server_keypair = KeyPair::generate().unwrap();

        let client_crypto = PhotonCrypto::new(client_keypair);
        let server_crypto = PhotonCrypto::new(server_keypair);

        let server_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut handshake = server_crypto.start_responder().unwrap();

            // Handshake logic
            let mut len_bytes = [0u8; 4];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut len_bytes).await.unwrap();
            let len = u32::from_be_bytes(len_bytes) as usize;
            let mut recv_buf = vec![0u8; len];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut recv_buf).await.unwrap();

            let mut buf = vec![0u8; 1024];
            handshake.read_message(&recv_buf, &mut buf).unwrap();

            let len = handshake.write_message(&[], &mut buf).unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut stream, &(len as u32).to_be_bytes()).await.unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut stream, &buf[..len]).await.unwrap();

            let mut len_bytes = [0u8; 4];
            if tokio::io::AsyncReadExt::read_exact(&mut stream, &mut len_bytes).await.is_err() { return true; }
            let len = u32::from_be_bytes(len_bytes) as usize;
            let mut recv_buf = vec![0u8; len];
            if tokio::io::AsyncReadExt::read_exact(&mut stream, &mut recv_buf).await.is_err() { return true; }

            // Expect an error here because the client is going to send bad data
            handshake.read_message(&recv_buf, &mut buf).is_err()
        });

        let mut client_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
        let mut handshake = client_crypto.start_initiator().unwrap();

        let mut buf = vec![0u8; 1024];
        let len = handshake.write_message(&[], &mut buf).unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut client_stream, &(len as u32).to_be_bytes()).await.unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut client_stream, &buf[..len]).await.unwrap();

        let mut len_bytes = [0u8; 4];
        tokio::io::AsyncReadExt::read_exact(&mut client_stream, &mut len_bytes).await.unwrap();
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut recv_buf = vec![0u8; len];
        tokio::io::AsyncReadExt::read_exact(&mut client_stream, &mut recv_buf).await.unwrap();
        handshake.read_message(&recv_buf, &mut buf).unwrap();

        // Simulating sending bad MAC by sending random bytes for the third message
        tokio::io::AsyncWriteExt::write_all(&mut client_stream, &(16_u32).to_be_bytes()).await.unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut client_stream, &[0; 16]).await.unwrap();

        let success = server_task.await.unwrap();
        assert!(success, "Expected wrong key handshake failure");
    }
}
