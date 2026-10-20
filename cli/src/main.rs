use clap::{Parser, Subcommand};
use photon_core::crypto::{KeyPair, PhotonCrypto, NOISE_PATTERN};
use photon_core::discovery::Discovery;
use photon_core::transfer::TransferManager;
use log::{info, error};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send files to a discovered device
    Send {
        /// Files to send
        #[arg(required = true)]
        files: Vec<String>,
    },
    /// Start listening for incoming connections
    Receive {
        /// Directory to save files
        #[arg(short, long, default_value = ".")]
        dir: String,

        /// Port to bind to
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let crypto = PhotonCrypto::new(keypair);
    let fingerprint = crypto.get_public_key_fingerprint();
    info!("My fingerprint: {}", fingerprint);

    match &cli.command {
        Commands::Send { files } => {
            info!("Sending files: {:?}", files);
            let discovery = Discovery::new()?;

            let (tx, mut rx) = tokio::sync::mpsc::channel(10);
            discovery.start_listening(tx).await?;

            info!("Scanning for devices... (Press Ctrl+C to exit)");
            if let Some(device) = rx.recv().await {
                info!("Found device: {} ({}:{})", device.name, device.ip, device.port);
                info!("Public Key Fingerprint: {}", device.pubkey_fingerprint);
                // Implementation for connecting and sending will go here
                // Left empty for M1 CLI scaffolding
            }
        }
        Commands::Receive { dir, port } => {
            let discovery = Discovery::new()?;
            let instance_name = format!("Photon-{}", std::process::id());

            info!("Starting receiver...");
            info!("Save directory: {}", dir);

            discovery.start_broadcasting(&instance_name, *port, &fingerprint)?;
            info!("Broadcasting mDNS service on port {}", port);

            let _manager = TransferManager::new(dir);
            // TCP listener logic will go here
            // Left empty for M1 CLI scaffolding

            tokio::signal::ctrl_c().await?;
            discovery.stop_broadcasting(&instance_name)?;
            info!("Shutting down receiver.");
        }
    }

    Ok(())
}
